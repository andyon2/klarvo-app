# Deep Modules + Vertical Slices

Quelle: AI-Hero-Research Delta-Analyse (2026-03-19)

- Code nach Features organisieren statt nach technischen Schichten — AI arbeitet praeziser wenn zusammengehoeriger Code beieinander liegt
- Deep Modules (Ousterhout): Einfache Interfaces, komplexe Implementierung dahinter — reduziert die Anzahl Dateien die der Agent gleichzeitig im Kontext halten muss
- AI-Genauigkeit: ~60% bei tight coupling → ~95% bei sauberen Modulen (Paul Simmering, Thoughtworks Radar)
- Fuer Rust/Tauri: Trait-basierte Module mit klaren Boundaries. Jedes Feature (z.B. Audio-Pipeline, Whisper-Integration) als eigenes Modul mit definiertem Public Interface
- Tracer Bullets: Bei neuen Features zuerst eine minimale End-to-End-Implementierung (UI → Backend → Persistence), dann ausbauen — erzwingt vertikale Slices

Vollstaendige Quelle: ~/claude-projects/project-builder/briefings/ai-hero-research/delta-analyse.md
