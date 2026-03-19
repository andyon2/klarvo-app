package com.dikta.voice

import android.Manifest
import android.accessibilityservice.AccessibilityServiceInfo
import android.app.AlertDialog
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.PowerManager
import android.provider.Settings
import android.util.Log
import android.view.accessibility.AccessibilityManager
import androidx.activity.enableEdgeToEdge
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat

/**
 * Entry point for the Dikta app on Android.
 *
 * Runs a sequential permission/setup chain on every onResume():
 *
 *   1. SYSTEM_ALERT_WINDOW   -- overlay permission (system settings screen)
 *   2. POST_NOTIFICATIONS    -- foreground-service notification (Android 13+)
 *   3. RECORD_AUDIO          -- microphone access
 *   4. Accessibility Service -- keyboard detection + auto-paste
 *                               shown as AlertDialog with direct settings link
 *   5. Battery Optimization  -- prevent Doze from killing the overlay service
 *                               shown as AlertDialog with direct settings link
 *   6. Start DiktaOverlayService
 *
 * Each step returns early after prompting the user. onResume() is called again
 * when the user returns from a system settings screen, which advances the chain.
 */
class MainActivity : TauriActivity() {

    companion object {
        private const val REQUEST_RECORD_AUDIO = 1001
        private const val REQUEST_POST_NOTIFICATIONS = 1002
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
    }

    override fun onResume() {
        super.onResume()
        checkPermissionsAndStart()
    }

    private fun checkPermissionsAndStart() {
        // Step 1: Overlay permission (must be granted via system settings screen).
        if (!Settings.canDrawOverlays(this)) {
            val intent = Intent(
                Settings.ACTION_MANAGE_OVERLAY_PERMISSION,
                Uri.parse("package:$packageName")
            )
            startActivity(intent)
            return
        }

        // Step 2: POST_NOTIFICATIONS runtime permission (Android 13+ / API 33+).
        // Required for the foreground-service notification to be visible.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            if (ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS)
                != PackageManager.PERMISSION_GRANTED
            ) {
                ActivityCompat.requestPermissions(
                    this,
                    arrayOf(Manifest.permission.POST_NOTIFICATIONS),
                    REQUEST_POST_NOTIFICATIONS
                )
                return
            }
        }

        // Step 3: Microphone runtime permission.
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO)
            != PackageManager.PERMISSION_GRANTED
        ) {
            ActivityCompat.requestPermissions(
                this,
                arrayOf(Manifest.permission.RECORD_AUDIO),
                REQUEST_RECORD_AUDIO
            )
            return
        }

        // Step 4: Accessibility Service -- required for system-wide keyboard detection
        // and auto-paste into focused text fields.
        // Not strictly blocking: the bubble still works without it (clipboard fallback +
        // reflection-based keyboard detection), but the experience is much better with it.
        if (!isAccessibilityServiceEnabled()) {
            AlertDialog.Builder(this)
                .setTitle("Enable Accessibility Access")
                .setMessage(
                    "Dikta uses the Accessibility Service to detect when the keyboard " +
                    "opens in any app so the voice bubble appears automatically.\n\n" +
                    "In the next screen, find \"Dikta\" under Installed Services and " +
                    "switch it on."
                )
                .setPositiveButton("Open Accessibility Settings") { _, _ ->
                    startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS))
                }
                .setNegativeButton("Skip for now") { _, _ ->
                    // Continue without accessibility; reflection fallback will handle it.
                    checkBatteryOptimization()
                }
                .setCancelable(false)
                .show()
            return
        }

        checkBatteryOptimization()
    }

    /**
     * Step 5: Battery optimization.
     *
     * Android's Doze mode can suspend network access and kill background services.
     * Asking for unrestricted battery usage ensures the overlay service and
     * Turso sync continue to work reliably even when the screen is off.
     *
     * We use ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS when available (API 23+),
     * which shows a system dialog. As a fallback (e.g. some OEM ROMs that block the
     * direct request) we fall back to opening the per-app battery settings page.
     */
    private fun checkBatteryOptimization() {
        val pm = getSystemService(POWER_SERVICE) as PowerManager
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M &&
            !pm.isIgnoringBatteryOptimizations(packageName)
        ) {
            AlertDialog.Builder(this)
                .setTitle("Unrestricted Battery Usage")
                .setMessage(
                    "For reliable keyboard detection and background sync, set Dikta's " +
                    "battery usage to \"Unrestricted\" (or \"No restrictions\").\n\n" +
                    "This prevents Android from putting the Dikta bubble to sleep."
                )
                .setPositiveButton("Open Battery Settings") { _, _ ->
                    // Try the direct ignore-battery-optimizations request first.
                    // Some OEMs (Xiaomi HyperOS, Samsung) may redirect this to their own
                    // power-management screen -- that's fine, the effect is the same.
                    val directIntent = Intent(
                        Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS,
                        Uri.parse("package:$packageName")
                    )
                    if (directIntent.resolveActivity(packageManager) != null) {
                        startActivity(directIntent)
                    } else {
                        // Fallback: open the per-app details page in system settings.
                        startActivity(
                            Intent(
                                Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                                Uri.parse("package:$packageName")
                            )
                        )
                    }
                }
                .setNegativeButton("Skip") { _, _ ->
                    startOverlayService()
                }
                .setCancelable(false)
                .show()
            return
        }

        startOverlayService()
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        when (requestCode) {
            REQUEST_POST_NOTIFICATIONS -> {
                // Continue the chain regardless of grant result -- the notification
                // is nice-to-have, but the bubble works without it.
                checkPermissionsAndStart()
            }
            REQUEST_RECORD_AUDIO -> {
                if (grantResults.isNotEmpty() && grantResults[0] == PackageManager.PERMISSION_GRANTED) {
                    // Re-run the permission chain to proceed to accessibility step.
                    checkPermissionsAndStart()
                }
                // If denied: do nothing. User can re-open the app to try again.
                // Without RECORD_AUDIO the dictation feature simply won't work.
            }
        }
    }

    /**
     * Returns true when DiktaAccessibilityService is active.
     *
     * Uses two methods to handle OEM variations (Xiaomi, Samsung use non-standard
     * separator formats or component name casing in ENABLED_ACCESSIBILITY_SERVICES):
     *
     *   Method 1 (primary): AccessibilityManager.getEnabledAccessibilityServiceList()
     *     Queries the live list of running accessibility services -- most reliable.
     *
     *   Method 2 (fallback): Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES string.
     *     Splits by ":" and compares using ComponentName.flattenToString() for
     *     case-insensitive matching, which handles OEM formatting quirks.
     */
    private fun isAccessibilityServiceEnabled(): Boolean {
        // Method 1: Query the live service list via AccessibilityManager.
        val am = getSystemService(Context.ACCESSIBILITY_SERVICE) as? AccessibilityManager
        if (am != null) {
            val runningServices = am.getEnabledAccessibilityServiceList(
                AccessibilityServiceInfo.FEEDBACK_ALL_MASK
            )
            for (info in runningServices) {
                if (info.resolveInfo.serviceInfo.packageName == packageName) {
                    Log.d("MainActivity", "Accessibility service confirmed via AccessibilityManager")
                    return true
                }
            }
        }

        // Method 2: Fallback -- parse the Settings.Secure string manually.
        // Split by ":" (standard separator) and compare with ComponentName to handle
        // OEMs that store entries in a different case or with extra whitespace.
        val expectedComponent = ComponentName(
            this,
            DiktaAccessibilityService::class.java
        ).flattenToString()
        val enabledString = Settings.Secure.getString(
            contentResolver,
            Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES
        ) ?: return false
        val found = enabledString.split(":").any {
            it.trim().equals(expectedComponent, ignoreCase = true)
        }
        if (found) {
            Log.d("MainActivity", "Accessibility service confirmed via Settings.Secure fallback")
        }
        return found
    }

    private fun startOverlayService() {
        val intent = Intent(this, DiktaOverlayService::class.java)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            startForegroundService(intent)
        } else {
            startService(intent)
        }
    }
}
