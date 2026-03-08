package com.dikta.voice

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.AccessibilityServiceInfo
import android.util.Log
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import android.view.accessibility.AccessibilityWindowInfo

/**
 * Accessibility service that detects when the soft keyboard is visible
 * and notifies DiktaOverlayService to show/hide the floating bubble.
 *
 * Detection strategy:
 *   Listen for TYPE_WINDOWS_CHANGED events, then walk the window list looking
 *   for a window of type AccessibilityWindowInfo.TYPE_INPUT_METHOD.
 *   This is far more reliable than reflection-based IMM polling and works
 *   system-wide across all apps (not just within our own process).
 *
 * Requirements:
 *   - FLAG_RETRIEVE_INTERACTIVE_WINDOWS: needed to access the windows list.
 *   - packageNames = null: receive events from ALL apps.
 *   - The user enables this service once in Android Settings > Accessibility.
 *     MainActivity guides the user there if the service is not yet active.
 *
 * Fallback:
 *   If this service is not active, DiktaOverlayService falls back to
 *   InputMethodManager.getInputMethodWindowVisibleHeight() reflection polling.
 */
class DiktaAccessibilityService : AccessibilityService() {

    companion object {
        private const val TAG = "DiktaAccess"
        /** Live reference to the running service; null when the service is not connected. */
        var instance: DiktaAccessibilityService? = null
    }

    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this
        Log.i(TAG, "AccessibilityService connected")

        // Reconfigure the service to monitor ALL apps (not just our own package).
        val info = serviceInfo ?: AccessibilityServiceInfo()
        info.eventTypes =
            AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED or
            AccessibilityEvent.TYPE_VIEW_FOCUSED or
            AccessibilityEvent.TYPE_WINDOWS_CHANGED
        info.feedbackType = AccessibilityServiceInfo.FEEDBACK_GENERIC
        info.flags = info.flags or
                AccessibilityServiceInfo.FLAG_REPORT_VIEW_IDS or
                AccessibilityServiceInfo.FLAG_RETRIEVE_INTERACTIVE_WINDOWS
        // null = monitor events from ALL packages, not just our own.
        info.packageNames = null
        info.notificationTimeout = 100
        serviceInfo = info
        Log.i(TAG, "Configured for system-wide keyboard detection")
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        if (event == null) return

        // Only re-check keyboard state on window-change events.
        // Checking on every event (e.g. TYPE_VIEW_FOCUSED spam) would be wasteful.
        if (event.eventType == AccessibilityEvent.TYPE_WINDOWS_CHANGED ||
            event.eventType == AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED
        ) {
            notifyKeyboardState()
        }
    }

    /**
     * Inspects the current window list for a window of type TYPE_INPUT_METHOD.
     * Calls DiktaOverlayService.onKeyboardVisibilityChanged() with the result.
     *
     * Must be called from the accessibility thread (which onAccessibilityEvent uses);
     * DiktaOverlayService.onKeyboardVisibilityChanged() posts to the main handler
     * internally, so cross-thread calls are safe.
     */
    private fun notifyKeyboardState() {
        val imeVisible = try {
            windows.any { it.type == AccessibilityWindowInfo.TYPE_INPUT_METHOD }
        } catch (e: Exception) {
            Log.w(TAG, "windows list unavailable", e)
            return
        }
        DiktaOverlayService.instance?.onKeyboardVisibilityChanged(imeVisible)
    }

    /**
     * Performs a paste action on the currently focused editable node.
     * Called by DiktaOverlayService after the transcription result is on the clipboard.
     */
    fun pasteIntoFocusedField() {
        val rootNode = rootInActiveWindow ?: return
        val focusedNode = findFocusedEditable(rootNode)
        focusedNode?.performAction(AccessibilityNodeInfo.ACTION_PASTE)
        focusedNode?.recycle()
        rootNode.recycle()
    }

    /**
     * Returns (a copy of) the first focused, editable node in the accessibility tree,
     * or null if none exists. Caller is responsible for recycling the returned node.
     */
    private fun findFocusedEditable(node: AccessibilityNodeInfo): AccessibilityNodeInfo? {
        if (node.isFocused && node.isEditable) return AccessibilityNodeInfo.obtain(node)
        for (i in 0 until node.childCount) {
            val child = node.getChild(i) ?: continue
            val result = findFocusedEditable(child)
            child.recycle()
            if (result != null) return result
        }
        return null
    }

    override fun onInterrupt() {
        // Required by AccessibilityService; nothing to interrupt here.
    }

    override fun onDestroy() {
        instance = null
        super.onDestroy()
    }
}
