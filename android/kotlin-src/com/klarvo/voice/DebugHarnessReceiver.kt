package com.klarvo.voice

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/**
 * DEBUG-ONLY manifest-declared broadcast receiver for the bubble state harness.
 *
 * Registered in src/debug/AndroidManifest.xml only — never present in release APKs (AC4).
 * Being manifest-declared means Android wakes the app process even when it is dead, which
 * solves the dynamic-receiver silent-drop problem: a dynamically registered receiver on a
 * dead process is never delivered to.
 *
 * On receipt, this receiver starts KlarvoOverlayService via startForegroundService() and
 * forwards the harness extras (state/rms/transcript) through the start Intent.
 * onStartCommand() applies the harness state once the service is initialised.
 *
 * The dynamic debugStateReceiver in KlarvoOverlayService is kept as the fast path for an
 * already-running process (avoids the Service-start round-trip overhead).
 *
 * adb usage (from cold-dead process):
 *   adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state idle -p com.klarvo.voice
 *   adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state recording --ef rms 0.8 --es transcript "Hallo Welt" -p com.klarvo.voice
 *   adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state transcribing --ef rms 0.3 --es transcript "Hallo Welt" -p com.klarvo.voice
 *   adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state done -p com.klarvo.voice
 */
class DebugHarnessReceiver : BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        if (!BuildConfig.DEBUG) return
        if (intent.action != ACTION_DEBUG_SET_STATE) return

        // Forward harness extras through the service start Intent so onStartCommand
        // can apply them after the service (and its bubbleView) have initialised.
        val serviceIntent = Intent(context, KlarvoOverlayService::class.java).apply {
            action = ACTION_DEBUG_SET_STATE
            // Carry through all harness extras unchanged.
            val state = intent.getStringExtra(EXTRA_STATE)
            if (state != null) putExtra(EXTRA_STATE, state)
            val rms = intent.getFloatExtra(EXTRA_RMS, -1f)
            if (rms >= 0f) putExtra(EXTRA_RMS, rms)
            val transcript = intent.getStringExtra(EXTRA_TRANSCRIPT)
            if (transcript != null) putExtra(EXTRA_TRANSCRIPT, transcript)
        }
        context.startForegroundService(serviceIntent)
    }

    companion object {
        // Mirror the constants from KlarvoOverlayService so this file is self-contained
        // (avoids making them internal/public in the service).
        const val ACTION_DEBUG_SET_STATE = "com.klarvo.voice.DEBUG_SET_STATE"
        const val EXTRA_STATE      = "state"
        const val EXTRA_RMS        = "rms"
        const val EXTRA_TRANSCRIPT = "transcript"
    }
}
