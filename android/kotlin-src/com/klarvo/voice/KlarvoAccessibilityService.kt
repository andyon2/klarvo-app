package com.klarvo.voice

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.AccessibilityServiceInfo
import android.os.Build
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import android.view.accessibility.AccessibilityWindowInfo
import android.view.inputmethod.InputMethodManager

/**
 * Accessibility service that detects when the soft keyboard is visible
 * and notifies KlarvoOverlayService to show/hide the floating bubble.
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
 *   If this service is not active, KlarvoOverlayService falls back to
 *   InputMethodManager.getInputMethodWindowVisibleHeight() reflection polling.
 */
class KlarvoAccessibilityService : AccessibilityService() {

    companion object {
        private const val TAG = "KlarvoAccess"
        /** Live reference to the running service; null when the service is not connected. */
        var instance: KlarvoAccessibilityService? = null
    }

    /**
     * Cached set of enabled IME package names. Built once in onServiceConnected and
     * refreshed on demand via refreshImePackageCache(). Avoids a system IPC call on
     * every TYPE_WINDOW_STATE_CHANGED event; the list is typically 1–3 entries and
     * changes only when the user installs or switches a keyboard.
     */
    private var imePackageCache: Set<String> = emptySet()

    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this
        refreshImePackageCache()
        KlarvoLogger.i(TAG,"AccessibilityService connected")

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
        KlarvoLogger.i(TAG,"Configured for system-wide keyboard detection")
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        if (event == null) return

        if (event.eventType == AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED) {
            // Check if the newly focused app is a banking/security app.
            // Exclude our own package to avoid accidentally blocking the Klarvo settings screen.
            val pkg = event.packageName?.toString()
            if (pkg != null && pkg != packageName) {
                val blocked = BankingAppBlocklist.isBlocked(pkg, this)
                if (blocked) {
                    // Always act on banking app detection immediately.
                    KlarvoOverlayService.instance?.onBankingAppStateChanged(true, pkg)
                } else if (!isSystemPackage(pkg) && !isInputMethod(pkg)) {
                    // Only clear banking state for real user-facing apps.
                    // System packages (android, systemui, launchers) fire window events
                    // when showing Toasts, system dialogs, etc. — do NOT let these
                    // accidentally reset bankingAppActive to false while a banking app is open.
                    // IME packages (Gboard, Samsung Keyboard, SwiftKey, etc.) fire a
                    // TYPE_WINDOW_STATE_CHANGED event when the keyboard opens — do NOT
                    // treat the keyboard gaining focus as "banking app left foreground".
                    KlarvoOverlayService.instance?.onBankingAppStateChanged(false, pkg)
                }
                // System packages and IME packages: silently ignore — don't change banking state at all.
            }
        }

        // Only re-check keyboard state on window-change events.
        // Checking on every event (e.g. TYPE_VIEW_FOCUSED spam) would be wasteful.
        if (event.eventType == AccessibilityEvent.TYPE_WINDOWS_CHANGED ||
            event.eventType == AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED
        ) {
            // checkForegroundBankingApp() runs first so banking state is set before
            // notifyKeyboardState() can trigger a showBubble() call.
            checkForegroundBankingApp()
            notifyKeyboardState()
        }
    }

    /**
     * Inspects the accessibility window list for the topmost APPLICATION window.
     * If its package is in the banking blocklist, signals onBankingAppStateChanged(true).
     *
     * This is a secondary detection path for banking apps (e.g. N26) that suppress
     * TYPE_WINDOW_STATE_CHANGED accessibility events as a security measure — the primary
     * event-driven path in onAccessibilityEvent() never fires for those apps.
     *
     * Design constraints:
     *   - Only SETS banking state, never clears it. Clearing via window inspection is
     *     unreliable because root can be null when the app blocks accessibility reads.
     *     Clearing continues to rely on TYPE_WINDOW_STATE_CHANGED from the next real app.
     *   - root.recycle() is mandatory — AccessibilityNodeInfo leaks if skipped.
     *   - Iterates ALL APPLICATION windows (not just the first) since some apps stack them.
     *   - Returns on first banking app found — no need to scan remaining windows.
     */
    private fun checkForegroundBankingApp() {
        try {
            for (window in windows) {
                if (window.type == AccessibilityWindowInfo.TYPE_APPLICATION) {
                    val root = window.root ?: continue
                    val pkg = root.packageName?.toString()
                    root.recycle()
                    if (pkg != null && pkg != packageName) {
                        if (BankingAppBlocklist.isBlocked(pkg, this)) {
                            KlarvoOverlayService.instance?.onBankingAppStateChanged(true, pkg)
                            return
                        }
                    }
                }
            }
        } catch (e: Exception) {
            KlarvoLogger.w(TAG, "checkForegroundBankingApp failed: ${e.message}")
        }
    }

    /**
     * Inspects the current window list for a window of type TYPE_INPUT_METHOD.
     * Calls KlarvoOverlayService.onKeyboardVisibilityChanged() with the result.
     *
     * Must be called from the accessibility thread (which onAccessibilityEvent uses);
     * KlarvoOverlayService.onKeyboardVisibilityChanged() posts to the main handler
     * internally, so cross-thread calls are safe.
     */
    private fun notifyKeyboardState() {
        val imeVisible = try {
            windows.any { it.type == AccessibilityWindowInfo.TYPE_INPUT_METHOD }
        } catch (e: Exception) {
            KlarvoLogger.w(TAG,"windows list unavailable", e)
            return
        }
        KlarvoOverlayService.instance?.onKeyboardVisibilityChanged(imeVisible)
    }

    /**
     * Performs a paste action on the currently focused editable node.
     * Called by KlarvoOverlayService after the transcription result is on the clipboard.
     */
    fun pasteIntoFocusedField() {
        val rootNode = rootInActiveWindow ?: return
        val focusedNode = findFocusedEditable(rootNode)
        focusedNode?.performAction(AccessibilityNodeInfo.ACTION_PASTE)
        focusedNode?.recycle()
        rootNode.recycle()
    }

    /**
     * Sends an Enter / Send action to the currently focused editable node.
     * Called by KlarvoOverlayService when auto-send is enabled for the active gesture.
     *
     * Implementation strategy:
     *   Primary: ACTION_IME_ENTER -- maps to the IME's action button (Send, Go, Search, etc.).
     *            Works in most chat apps (WhatsApp, Telegram, Signal) when the field
     *            has imeOptions set to actionSend.
     *   Fallback: ACTION_NEXT_AT_MOVEMENT_GRANULARITY is NOT used here -- not useful.
     *            There is no universal "press Enter key" via AccessibilityService.
     *
     * Limitation: Apps that implement a custom send button (not tied to IME action)
     * may not respond to ACTION_IME_ENTER. In those cases the user must tap Send manually.
     * This is documented behavior -- no workaround without knowing the specific app layout.
     */
    fun performEnter() {
        val rootNode = rootInActiveWindow ?: run {
            KlarvoLogger.w(TAG,"performEnter: rootInActiveWindow is null")
            return
        }
        val focusedNode = findFocusedEditable(rootNode)
        if (focusedNode != null) {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                // API 30+ — use the proper IME Enter action
                val performed = focusedNode.performAction(
                    AccessibilityNodeInfo.AccessibilityAction.ACTION_IME_ENTER.id
                )
                if (!performed) {
                    KlarvoLogger.d(TAG,"performEnter: ACTION_IME_ENTER returned false (app may not support it)")
                }
            } else {
                KlarvoLogger.d(TAG,"performEnter: ACTION_IME_ENTER requires API 30+, skipping")
            }
            focusedNode.recycle()
        } else {
            KlarvoLogger.d(TAG,"performEnter: no focused editable node found")
        }
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

    /**
     * Rebuilds imePackageCache from the system's enabled input method list.
     * Call once in onServiceConnected; the list only changes when the user installs
     * or switches keyboards, so a single build per service lifetime is sufficient.
     */
    private fun refreshImePackageCache() {
        try {
            val imm = getSystemService(INPUT_METHOD_SERVICE) as InputMethodManager
            imePackageCache = imm.enabledInputMethodList.map { it.packageName }.toSet()
            KlarvoLogger.d(TAG, "IME package cache: $imePackageCache")
        } catch (e: Exception) {
            KlarvoLogger.w(TAG, "Failed to build IME package cache", e)
            imePackageCache = emptySet()
        }
    }

    /**
     * Returns true if [pkg] is an enabled input method (on-screen keyboard).
     *
     * IME packages (e.g. com.google.android.inputmethod.latin for Gboard,
     * com.samsung.android.honeyboard, com.swiftkey.inputmethod) fire a
     * TYPE_WINDOW_STATE_CHANGED event when the keyboard opens. Without this check,
     * the keyboard gaining focus would be treated as "user left banking app" and
     * would incorrectly re-show the floating bubble.
     *
     * Uses imePackageCache to avoid an IPC call on every accessibility event.
     */
    private fun isInputMethod(pkg: String): Boolean = imePackageCache.contains(pkg)

    /**
     * Returns true for packages that represent transient system UI (Toasts, dialogs, system
     * overlays, launchers). These packages fire TYPE_WINDOW_STATE_CHANGED events during normal
     * interaction but are never the "real" foreground app from the user's perspective.
     *
     * We must NOT use these events to reset bankingAppActive, because:
     *   - A Toast shown on top of a banking app triggers a "android" package event.
     *   - That would immediately clear the banking block and re-show the bubble.
     *
     * OEM launchers are included because switching home-screen layouts briefly surfaces the
     * launcher package before the target app registers a window event.
     */
    private fun isSystemPackage(pkg: String): Boolean {
        return pkg == "android" ||
               pkg.startsWith("com.android.systemui") ||
               pkg.startsWith("com.android.launcher") ||
               pkg.startsWith("com.google.android.apps.nexuslauncher") ||
               pkg.startsWith("com.sec.android.app.launcher") ||   // Samsung One UI launcher
               pkg.startsWith("com.miui.home") ||                  // Xiaomi MIUI launcher
               pkg.startsWith("com.huawei.android.launcher") ||    // Huawei EMUI launcher
               pkg.startsWith("com.oppo.launcher") ||              // OPPO launcher
               pkg.startsWith("com.vivo.launcher")                 // Vivo launcher
    }

    override fun onInterrupt() {
        // Required by AccessibilityService; nothing to interrupt here.
    }

    override fun onDestroy() {
        instance = null
        super.onDestroy()
    }
}
