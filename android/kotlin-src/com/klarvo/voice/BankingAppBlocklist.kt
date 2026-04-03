package com.klarvo.voice

/**
 * Curated blocklist of banking, password-manager, and security apps whose
 * "screen overlay detected" dialog would block the user.
 *
 * The Klarvo bubble (SYSTEM_ALERT_WINDOW) is automatically hidden when any
 * of these apps is in the foreground, and restored when the user navigates away.
 *
 * This is a security feature and cannot be disabled by the user.
 * Users CAN add custom entries via Settings (stored in SharedPreferences).
 */
object BankingAppBlocklist {

    /** Built-in blocklist. Sorted alphabetically by package name. */
    private val BUILTIN = setOf(
        // --- Germany (DACH) ---
        "com.starfinanz.smob.android.sfinanzstatus",  // Sparkasse
        "de.fiducia.smartphone.android.banking.vr",    // VR Banking
        "com.db.pwcc.dbmobile",                        // Deutsche Bank
        "de.commerzbanking.mobil",                     // Commerzbank
        "de.ingdiba.bankingapp",                       // ING
        "de.dkb.portalapp",                            // DKB
        "com.n26.android",                              // N26
        "de.comdirect.app",                            // comdirect
        "com.consorsbank",                              // Consorsbank
        "de.hypovereinsbank.android.banking",           // HyperVereinsbank
        "de.postbank.finanzassistent",                  // Postbank
        "de.santander.presentation",                    // Santander DE
        "piuk.blockchain.android",                      // Blockchain.com
        "de.number26.android",                          // N26 (old package)
        // --- Austria ---
        "at.easybank.mbanking",                         // easybank
        "at.spardat.netbanking",                        // George (Erste Bank)
        "at.bawag.mbanking",                            // BAWAG
        // --- Switzerland ---
        "ch.postfinance.android",                       // PostFinance
        "com.ubs.swidKXJ.android",                     // UBS
        "com.zuercher.zkmb",                            // ZKB
        // --- US ---
        "com.chase.sig.android",                        // Chase
        "com.wf.wellsfargomobile",                     // Wells Fargo
        "com.bankofamerica.cashpromobile",              // Bank of America
        "com.citi.citimobile",                          // Citi
        "com.usbank.mobilebanking",                    // US Bank
        "com.ally.MobileBank",                         // Ally
        "com.capitalone.mobile.ui",                    // Capital One (not banking)
        "com.capitalone.mobileBanking",                // Capital One Banking
        "com.schwab.mobile",                            // Charles Schwab
        "com.americanexpress.android.acctsvcs.us",     // Amex
        // --- UK ---
        "com.barclays.android.barclaysmobilebanking",  // Barclays
        "uk.co.hsbc.hsbcukmobilebanking",              // HSBC
        "com.grfrg.revolutandroidmain",                // Revolut (note: not com.revolut)
        "com.revolut.revolut",                          // Revolut
        // --- Password Managers ---
        "com.onepassword.android",                      // 1Password (old)
        "com.onepassword7.android",                     // 1Password 7
        "com.agilebits.onepassword",                    // 1Password 8
        "com.lastpass.lpandroid",                       // LastPass
        "com.x8bit.bitwarden",                          // Bitwarden
        "com.dashlane",                                 // Dashlane
        "keepass2android.keepass2android",               // KeePass2Android
        "com.kunzisoft.keepass.free",                   // KeePassDX
        "org.kp2a.kp2a",                                // Keepass2Android Offline
        // --- Security / 2FA ---
        "com.google.android.apps.authenticator2",       // Google Authenticator
        "com.authy.authy",                              // Authy
        "org.fedorahosted.freeotp",                     // FreeOTP
        "com.azure.authenticator",                      // Microsoft Authenticator
        // --- Payment ---
        "com.paypal.android.p2pmobile",                // PayPal
        "de.klarna.app",                                // Klarna
    )

    /** Returns the full active blocklist (built-in + user-custom entries). */
    fun getBlocklist(context: android.content.Context): Set<String> {
        val custom = getCustomEntries(context)
        return if (custom.isEmpty()) BUILTIN else BUILTIN + custom
    }

    /** Returns true if the given package should trigger bubble hide. */
    fun isBlocked(packageName: String, context: android.content.Context): Boolean {
        return packageName in BUILTIN || packageName in getCustomEntries(context)
    }

    // --- User-custom entries (stored in SharedPreferences) ---

    private const val PREFS_NAME = "klarvo_banking_blocklist"
    private const val KEY_CUSTOM = "custom_packages"

    fun getCustomEntries(context: android.content.Context): Set<String> {
        return context.getSharedPreferences(PREFS_NAME, android.content.Context.MODE_PRIVATE)
            .getStringSet(KEY_CUSTOM, emptySet()) ?: emptySet()
    }

    fun addCustomEntry(context: android.content.Context, packageName: String) {
        val prefs = context.getSharedPreferences(PREFS_NAME, android.content.Context.MODE_PRIVATE)
        val current = prefs.getStringSet(KEY_CUSTOM, emptySet())?.toMutableSet() ?: mutableSetOf()
        current.add(packageName.trim())
        prefs.edit().putStringSet(KEY_CUSTOM, current).apply()
    }

    fun removeCustomEntry(context: android.content.Context, packageName: String) {
        val prefs = context.getSharedPreferences(PREFS_NAME, android.content.Context.MODE_PRIVATE)
        val current = prefs.getStringSet(KEY_CUSTOM, emptySet())?.toMutableSet() ?: mutableSetOf()
        current.remove(packageName.trim())
        prefs.edit().putStringSet(KEY_CUSTOM, current).apply()
    }
}
