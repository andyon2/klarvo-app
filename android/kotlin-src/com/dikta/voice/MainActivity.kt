package com.dikta.voice

import android.Manifest
import android.app.AlertDialog
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.PowerManager
import android.provider.Settings
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
     * Returns true when DiktaAccessibilityService is listed in the system's enabled services.
     * The enabled-services string uses "package/fully.qualified.ClassName" format.
     */
    private fun isAccessibilityServiceEnabled(): Boolean {
        val service = "$packageName/${DiktaAccessibilityService::class.java.canonicalName}"
        val enabledServices = Settings.Secure.getString(
            contentResolver,
            Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES
        ) ?: return false
        return enabledServices.contains(service)
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
