---
name: scaffold
description: Erstellt ein neues Modul oder eine neue Komponente aus Template. Aufrufen mit Typ und Name, z.B. "/scaffold rust-module paste" oder "/scaffold react-component Overlay".
argument-hint: "[typ] [name] -- typ: rust-module | react-component | android-service"
allowed-tools: Read, Write, Bash, Glob
context: fork
model: haiku
---

Erstelle ein neues Modul/Component fuer das Voxlit-Projekt.

## Argumente parsen

Aus `$ARGUMENTS` extrahiere:
- **typ**: `rust-module` | `react-component` | `android-service`
- **name**: Name des Moduls/Components (z.B. "paste", "Overlay", "VoiceInputService")

## Vorgehensweise

### Wenn typ = `rust-module`

1. Erstelle Verzeichnis `src-tauri/src/[name]/`
2. Erstelle `src-tauri/src/[name]/mod.rs`:
   ```rust
   //! [Name] module for Voxlit
   //!
   //! TODO: Add module description

   mod error;

   pub use error::*;

   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn it_works() {
           // TODO: Add tests
       }
   }
   ```
3. Erstelle `src-tauri/src/[name]/error.rs`:
   ```rust
   use thiserror::Error;

   #[derive(Error, Debug)]
   pub enum [Name]Error {
       #[error("Not implemented")]
       NotImplemented,
   }
   ```
4. Fuege `mod [name];` in `src-tauri/src/main.rs` oder `src-tauri/src/lib.rs` hinzu (pruefe was existiert)
5. Melde: "Rust-Modul `[name]` erstellt in src-tauri/src/[name]/. Naechster Schritt: Implementierung durch rust-core Agent."

### Wenn typ = `react-component`

1. Erstelle `src/components/[Name].tsx`:
   ```tsx
   import { FC } from 'react';

   interface [Name]Props {
     // TODO: Define props
   }

   export const [Name]: FC<[Name]Props> = (props) => {
     return (
       <div className="TODO">
         {/* TODO: Implement [Name] */}
       </div>
     );
   };
   ```
2. Fuege Export in `src/components/index.ts` hinzu (erstelle die Datei falls noetig)
3. Melde: "React-Component `[Name]` erstellt in src/components/[Name].tsx. Naechster Schritt: Implementierung durch ui-dev Agent."

### Wenn typ = `android-service`

1. Ermittle das Package aus bestehenden Kotlin-Dateien in `android/` oder nutze `com.voxlit.app`
2. Erstelle die Kotlin-Datei im passenden Verzeichnis:
   ```kotlin
   package com.voxlit.app

   // TODO: Implement [Name]
   class [Name] {
   }
   ```
3. Melde: "Android-Service `[Name]` erstellt. Naechster Schritt: Implementierung durch android-platform Agent."

## Wenn typ nicht erkannt wird

Melde: "Unbekannter Typ `[typ]`. Verfuegbare Typen: rust-module, react-component, android-service."
