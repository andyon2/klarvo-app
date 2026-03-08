package com.dikta.voice

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.widget.Toast
import androidx.activity.enableEdgeToEdge
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat

class MainActivity : TauriActivity() {

    companion object {
        private const val REQUEST_RECORD_AUDIO = 1001
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

        // Step 2: Microphone runtime permission.
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

        // Step 3 (optional): Hint about accessibility service for auto-paste.
        // Not blocking -- the bubble works without it (clipboard fallback).
        if (!isAccessibilityServiceEnabled()) {
            Toast.makeText(
                this,
                "Tip: Enable Dikta in Accessibility Settings for auto-paste into text fields.",
                Toast.LENGTH_LONG
            ).show()
        }

        // Start the overlay service -- bubble is always visible.
        startOverlayService()
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == REQUEST_RECORD_AUDIO && grantResults.isNotEmpty()
            && grantResults[0] == PackageManager.PERMISSION_GRANTED
        ) {
            // Re-run the permission chain so we also check accessibility before starting.
            checkPermissionsAndStart()
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
