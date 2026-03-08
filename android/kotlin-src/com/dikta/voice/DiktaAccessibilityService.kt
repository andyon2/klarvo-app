package com.dikta.voice

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.AccessibilityServiceInfo
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo

/**
 * Accessibility service that detects when the soft keyboard is visible.
 *
 * Used to:
 *   1. Show/hide the floating bubble when the keyboard appears/disappears.
 *   2. Paste transcribed text directly into the focused field after dictation.
 *
 * Detection strategy: Check the system window list for a window of type
 * TYPE_INPUT_METHOD. This is far more reliable than traversing the accessibility
 * tree looking for focused editable nodes (which varies wildly across apps).
 *
 * Requires FLAG_RETRIEVE_INTERACTIVE_WINDOWS to access the windows list.
 *
 * The user must enable this service once in Android Settings > Accessibility.
 * MainActivity guides the user there if it is not yet enabled.
 */
class DiktaAccessibilityService : AccessibilityService() {

    companion object {
        /** Live reference to the running service; null when the service is not connected. */
        var instance: DiktaAccessibilityService? = null
    }

    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this
        // Reconfigure the service to monitor ALL apps (not just our own package).
        val info = serviceInfo ?: AccessibilityServiceInfo()
        info.eventTypes = AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED or
                AccessibilityEvent.TYPE_VIEW_FOCUSED or
                AccessibilityEvent.TYPE_WINDOWS_CHANGED
        info.feedbackType = AccessibilityServiceInfo.FEEDBACK_GENERIC
        info.flags = info.flags or
                AccessibilityServiceInfo.FLAG_REPORT_VIEW_IDS or
                AccessibilityServiceInfo.FLAG_RETRIEVE_INTERACTIVE_WINDOWS
        // Explicitly null = monitor events from ALL packages, not just our own.
        info.packageNames = null
        info.notificationTimeout = 100
        serviceInfo = info
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        // Bubble visibility is no longer managed by this service (always visible).
        // This handler is kept alive so the service stays connected and
        // pasteIntoFocusedField() can be called after dictation.
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
