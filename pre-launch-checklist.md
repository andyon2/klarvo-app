# Pre-Launch Checklist -- Klarvo v1.0

Erstellt: 2026-03-25
Status: In Planung. Kein Task gestartet.

---

## Features (muessen vor v1.0 rein)

- [ ] **i18n (DE + EN)** -- react-i18next, JSON-Dateien, Language Switcher, alle Komponenten. Plan: `briefings/plan-i18n.md`. Aufwand: Gross (3-4 Sessions).
- [ ] **Light Theme** -- Zweites Farbset, Theme-Toggle in Settings, Praeferenz persistieren. Aufwand: Mittel (1 Session).
- [ ] **Lemon Squeezy License API** -- Dual-System: HMAC (lokal, Tester) + LS (Kunden, Online-Aktivierung). Activation Limit 3 Geraete. In-App Deactivate Button. Architektur: `knowledge/architecture.md` Sektion "License-Key-System". Aufwand: Gross (2-3 Sessions).
- [ ] **3-4 Hotkey-Slots** -- Aktuell 2 Slots. Erweitern auf 3-4 mit je eigenem Modus + Auto-Send-Toggle. Aufwand: Klein-Mittel.
- [ ] **Auto-Send-Indikator in FloatingBar** -- Kleines Icon/Badge das anzeigt ob Auto-Send aktiv ist. Aufwand: Klein.

## Polish (UX-Hygiene vor v1.0)

- [ ] **Features ausblenden** -- App Profiles und Integrations aus dem UI entfernen (ungetestet / nicht gebaut). Aufwand: Klein.
- [ ] **Advanced Settings hinter Collapse** -- Standardmaessig zugeklappt, Expander fuer Power-User. Aufwand: Klein.
- [ ] **Feedback-Link in Settings** -- Oeffnet GitHub Issues Seite (voxlit-app). Aufwand: Klein.
- [ ] **Onboarding-Review** -- Kommt ein Erstnutzer in 60s zum ersten Diktat? Testen und ggf. verbessern. Aufwand: Mittel.
- [ ] **Error-States Audit** -- Kein stilles Scheitern. Alle kritischen Fehlerpfade pruefen (kein API-Key, Mikro weg, Netzwerk weg, Rate Limit). Aufwand: Mittel.

## QA (vor Release)

- [ ] **QA-Checkliste erstellen** -- Jedes Feature als manueller Testfall mit konkreten Schritten.
- [ ] **RC-Zyklus** -- RC1 bauen → testen → Blocker fixen → RC2 → ... bis kein Blocker mehr.
- [ ] **i18n-Lint** -- Jeder Key in en.json hat Pendant in de.json. Kein fehlender String.

## Non-Code (Launch-Infrastruktur)

- [ ] **Landingpage klarvo.app** -- Andy baut erste Version.
- [ ] **Lemon Squeezy Account + Produkt** -- "Klarvo License", EUR 29 Early Bird, Checkout-Link.
- [ ] **Social Preview** -- Assets fuer GitHub, Twitter, Product Hunt.
- [ ] **README auf voxlit-app** -- Aktuell, gegen Code verifiziert.

---

## Quellen

- UX-Audit: `briefings/launch-ux-audit.md`
- i18n-Plan: `briefings/plan-i18n.md`
- Lizenz-Architektur: `knowledge/architecture.md` Sektion "License-Key-System: Dual"
- Produkt-Strategie: `knowledge/product-strategy.md`
