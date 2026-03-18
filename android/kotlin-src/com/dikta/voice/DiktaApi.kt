package com.dikta.voice

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.os.Build
import android.util.Log
import org.json.JSONObject
import org.json.JSONArray
import java.io.*
import java.net.HttpURLConnection
import java.net.URL
import java.util.concurrent.Callable
import java.util.concurrent.Executors

/**
 * API client for Groq Whisper STT and DeepSeek text cleanup.
 * Uses java.net.HttpURLConnection -- no extra dependencies needed.
 * All methods throw IOException on failure -- caller handles errors.
 */
object DiktaApi {

    data class Config(
        val groqApiKey: String,
        val deepseekApiKey: String,
        val language: String,
        val cleanupStyle: String,
        val tursoUrl: String,
        val tursoToken: String,
        val deviceId: String,
        val bubbleSize: Float = 1.0f,
        val bubbleOpacity: Float = 0.85f,
        // Kept for backwards compatibility -- no longer used in overlay logic.
        val bubbleRecordingMode: String = "hold",
        // Per-gesture recording controls (tap and long-press independently configured).
        val bubbleTapMode: String = "toggle",
        val bubbleTapAutoSend: Boolean = false,
        val bubbleTapSilenceSecs: Float = 2.0f,
        val bubbleLongPressMode: String = "hold",
        val bubbleLongPressAutoSend: Boolean = false,
        val bubbleLongPressSilenceSecs: Float = 2.0f
    )

    /**
     * Returns the app's data directory path.
     * Tauri writes config.json to app_data_dir() which maps to activity.dataDir
     * (i.e. /data/data/<package>/), NOT to context.filesDir.
     * API 24+ has context.dataDir; below that we use applicationInfo.dataDir.
     */
    private fun getDataDir(context: Context): File {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            context.dataDir
        } else {
            File(context.applicationInfo.dataDir)
        }
    }

    /**
     * Reads config.json from the app's data directory.
     * Tauri's app_data_dir() resolves to dataDir, not filesDir.
     * Returns null if the file doesn't exist or keys are missing.
     */
    fun readConfig(context: Context): Config? {
        val configFile = File(getDataDir(context), "config.json")
        if (!configFile.exists()) return null

        return try {
            val json = JSONObject(configFile.readText())
            val groqKey = json.optString("groqApiKey", "")
            val deepseekKey = json.optString("deepseekApiKey", "")
            val language = json.optString("language", "")
            val cleanupStyle = json.optString("cleanupStyle", "polished")
            val tursoUrl = json.optString("tursoUrl", "")
            val tursoToken = json.optString("tursoToken", "")
            val deviceId = json.optString("deviceId", "")
            val bubbleSize = json.optDouble("bubbleSize", 1.0).toFloat()
            val bubbleOpacity = json.optDouble("bubbleOpacity", 0.85).toFloat()
            // Rust serializes with camelCase (rename_all on AppConfig struct).
            val bubbleRecordingMode = json.optString("bubbleRecordingMode", "hold")
            // Per-gesture controls (tap and long-press independently configured).
            val bubbleTapMode = json.optString("bubbleTapMode", "toggle")
            val bubbleTapAutoSend = json.optBoolean("bubbleTapAutoSend", false)
            val bubbleTapSilenceSecs = json.optDouble("bubbleTapSilenceSecs", 2.0).toFloat()
            val bubbleLongPressMode = json.optString("bubbleLongPressMode", "hold")
            val bubbleLongPressAutoSend = json.optBoolean("bubbleLongPressAutoSend", false)
            val bubbleLongPressSilenceSecs = json.optDouble("bubbleLongPressSilenceSecs", 2.0).toFloat()
            Log.d("DiktaApi", "readConfig: bubbleTapMode=$bubbleTapMode, bubbleLongPressMode=$bubbleLongPressMode, json has keys: ${json.keys().asSequence().filter { it.contains("bubble", ignoreCase = true) }.toList()}")

            if (groqKey.isBlank() && deepseekKey.isBlank()) null
            else Config(
                groqKey, deepseekKey, language, cleanupStyle, tursoUrl, tursoToken, deviceId,
                bubbleSize, bubbleOpacity, bubbleRecordingMode,
                bubbleTapMode, bubbleTapAutoSend, bubbleTapSilenceSecs,
                bubbleLongPressMode, bubbleLongPressAutoSend, bubbleLongPressSilenceSecs
            )
        } catch (e: Exception) {
            null
        }
    }

    /**
     * Saves a transcription entry to the SQLite history database.
     * Uses the same schema as the Rust/Tauri desktop app so history is shared.
     * Includes uuid, device_id, and synced columns for Turso cross-device sync.
     *
     * @param context   Android context (used to resolve the DB path)
     * @param finalText Cleaned/final text shown to the user
     * @param rawText   Raw transcript before LLM cleanup
     * @param style     Cleanup style (e.g. "polished", "verbatim", "chat")
     * @param language  Language code or empty string for auto-detect
     * @param deviceId  Device identifier for sync tracking (empty string if not configured)
     */
    fun saveToHistory(
        context: Context,
        finalText: String,
        rawText: String,
        style: String,
        language: String,
        deviceId: String = ""
    ) {
        val uuid = java.util.UUID.randomUUID().toString()
        val dbFile = File(getDataDir(context), "history.db")
        var db: SQLiteDatabase? = null
        try {
            db = SQLiteDatabase.openOrCreateDatabase(dbFile, null)
            db.execSQL(
                """
                CREATE TABLE IF NOT EXISTS history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    text TEXT NOT NULL,
                    raw_text TEXT,
                    style TEXT NOT NULL DEFAULT 'polished',
                    language TEXT NOT NULL DEFAULT '',
                    is_note INTEGER NOT NULL DEFAULT 0,
                    app_name TEXT,
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    uuid TEXT,
                    device_id TEXT,
                    synced INTEGER NOT NULL DEFAULT 0
                )
                """.trimIndent()
            )
            // Migrate existing tables that predate sync columns (best-effort).
            for (col in listOf(
                "uuid TEXT",
                "device_id TEXT",
                "synced INTEGER NOT NULL DEFAULT 0"
            )) {
                try { db.execSQL("ALTER TABLE history ADD COLUMN $col") } catch (_: Exception) {}
            }
            val stmt = db.compileStatement(
                "INSERT INTO history (text, raw_text, style, language, is_note, app_name, uuid, device_id, synced) VALUES (?, ?, ?, ?, 0, NULL, ?, ?, 0)"
            )
            stmt.bindString(1, finalText)
            stmt.bindString(2, rawText)
            stmt.bindString(3, style)
            stmt.bindString(4, language)
            stmt.bindString(5, uuid)
            stmt.bindString(6, deviceId)
            stmt.executeInsert()
        } catch (_: Exception) {
            // History saving is best-effort; never crash the main flow.
        } finally {
            try { db?.close() } catch (_: Exception) {}
        }
    }

    /**
     * Pushes unsynced history entries to Turso via the HTTP pipeline API.
     * Sync is best-effort: any failure is silently ignored.
     * Marks entries as synced (synced=1) only after a successful HTTP 2xx response.
     *
     * @param context     Android context
     * @param tursoUrl    Turso database URL (libsql:// or https://)
     * @param tursoToken  Turso auth token
     */
    fun pushToTurso(context: Context, tursoUrl: String, tursoToken: String) {
        if (tursoUrl.isBlank() || tursoToken.isBlank()) return

        val dbFile = File(getDataDir(context), "history.db")
        if (!dbFile.exists()) return

        var db: SQLiteDatabase? = null
        try {
            db = SQLiteDatabase.openOrCreateDatabase(dbFile, null)

            // Read unsynced entries that have a uuid (entries before the migration may lack one).
            val cursor = db.rawQuery(
                "SELECT uuid, text, raw_text, style, language, is_note, app_name, device_id, created_at FROM history WHERE synced = 0 AND uuid IS NOT NULL",
                null
            )

            if (cursor.count == 0) {
                cursor.close()
                return
            }

            val httpsUrl = tursoUrl.replace("libsql://", "https://")

            // Ensure the remote table exists before inserting rows.
            ensureRemoteTable(httpsUrl, tursoToken)

            val requests = JSONArray()
            val uuids = mutableListOf<String>()

            while (cursor.moveToNext()) {
                val entryUuid = cursor.getString(0)
                uuids.add(entryUuid)

                val args = JSONArray().apply {
                    put(JSONObject().put("type", "text").put("value", entryUuid))
                    put(JSONObject().put("type", "text").put("value", cursor.getString(1))) // text
                    if (cursor.isNull(2)) put(JSONObject().put("type", "null"))
                    else put(JSONObject().put("type", "text").put("value", cursor.getString(2))) // raw_text
                    put(JSONObject().put("type", "text").put("value", cursor.getString(3))) // style
                    put(JSONObject().put("type", "text").put("value", cursor.getString(4))) // language
                    put(JSONObject().put("type", "integer").put("value", cursor.getInt(5).toString())) // is_note
                    if (cursor.isNull(6)) put(JSONObject().put("type", "null"))
                    else put(JSONObject().put("type", "text").put("value", cursor.getString(6))) // app_name
                    if (cursor.isNull(7)) put(JSONObject().put("type", "null"))
                    else put(JSONObject().put("type", "text").put("value", cursor.getString(7))) // device_id
                    put(JSONObject().put("type", "text").put("value", cursor.getString(8))) // created_at
                }

                requests.put(JSONObject().apply {
                    put("type", "execute")
                    put("stmt", JSONObject().apply {
                        put("sql", "INSERT OR IGNORE INTO history (uuid, text, raw_text, style, language, is_note, app_name, device_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
                        put("args", args)
                    })
                })
            }
            cursor.close()

            requests.put(JSONObject().put("type", "close"))

            val body = JSONObject().put("requests", requests).toString().toByteArray(Charsets.UTF_8)
            val url = URL("$httpsUrl/v2/pipeline")
            val conn = url.openConnection() as HttpURLConnection
            conn.requestMethod = "POST"
            conn.doOutput = true
            conn.connectTimeout = 10_000
            conn.readTimeout = 15_000
            conn.setRequestProperty("Authorization", "Bearer $tursoToken")
            conn.setRequestProperty("Content-Type", "application/json")
            conn.outputStream.use { it.write(body) }

            if (conn.responseCode in 200..299) {
                // Mark successfully pushed entries as synced.
                for (uuid in uuids) {
                    db.execSQL("UPDATE history SET synced = 1 WHERE uuid = ?", arrayOf(uuid))
                }
            }
            conn.disconnect()

        } catch (_: Exception) {
            // Sync is best-effort -- never crash the main flow.
        } finally {
            try { db?.close() } catch (_: Exception) {}
        }
    }

    /**
     * Creates the history table in the remote Turso database if it does not exist yet.
     * The remote schema uses uuid as PRIMARY KEY (no local autoincrement id).
     */
    private fun ensureRemoteTable(httpsUrl: String, token: String) {
        val requests = JSONArray().apply {
            put(JSONObject().apply {
                put("type", "execute")
                put("stmt", JSONObject().apply {
                    put("sql", """CREATE TABLE IF NOT EXISTS history (
                        uuid TEXT PRIMARY KEY,
                        text TEXT NOT NULL,
                        raw_text TEXT,
                        style TEXT NOT NULL DEFAULT 'polished',
                        language TEXT NOT NULL DEFAULT '',
                        is_note INTEGER NOT NULL DEFAULT 0,
                        app_name TEXT,
                        device_id TEXT,
                        created_at TEXT NOT NULL DEFAULT (datetime('now'))
                    )""")
                })
            })
            put(JSONObject().put("type", "close"))
        }

        val body = JSONObject().put("requests", requests).toString().toByteArray(Charsets.UTF_8)
        val url = URL("$httpsUrl/v2/pipeline")
        val conn = url.openConnection() as HttpURLConnection
        conn.requestMethod = "POST"
        conn.doOutput = true
        conn.connectTimeout = 10_000
        conn.readTimeout = 10_000
        conn.setRequestProperty("Authorization", "Bearer $token")
        conn.setRequestProperty("Content-Type", "application/json")
        conn.outputStream.use { it.write(body) }
        conn.responseCode  // wait for response
        conn.disconnect()
    }

    /**
     * Transcribes WAV audio using Groq Whisper API.
     * Sends multipart/form-data POST to Groq's transcription endpoint.
     *
     * @param wavBytes Raw WAV file bytes
     * @param apiKey   Groq API key
     * @param language Language code (e.g. "de", "en") or empty for auto-detect
     * @return Transcribed text
     * @throws IOException on network or API errors
     */
    fun transcribe(wavBytes: ByteArray, apiKey: String, language: String): String {
        val boundary = "----DiktaBoundary" + System.currentTimeMillis()
        val url = URL("https://api.groq.com/openai/v1/audio/transcriptions")
        val conn = url.openConnection() as HttpURLConnection

        try {
            conn.requestMethod = "POST"
            conn.doOutput = true
            conn.connectTimeout = 15_000
            conn.readTimeout = 30_000
            conn.setRequestProperty("Authorization", "Bearer $apiKey")
            conn.setRequestProperty("Content-Type", "multipart/form-data; boundary=$boundary")

            val body = buildMultipartBody(boundary, wavBytes, language)
            conn.setRequestProperty("Content-Length", body.size.toString())

            conn.outputStream.use { it.write(body) }

            val responseCode = conn.responseCode
            if (responseCode != 200) {
                val errorBody = conn.errorStream?.bufferedReader()?.readText() ?: "unknown error"
                throw IOException("Groq STT failed: HTTP $responseCode -- $errorBody")
            }

            val responseText = conn.inputStream.bufferedReader().readText()
            val json = JSONObject(responseText)
            return json.getString("text").trim()
        } finally {
            conn.disconnect()
        }
    }

    /**
     * Cleans up dictation text using DeepSeek chat API.
     *
     * @param text   Raw transcription text to clean up
     * @param apiKey DeepSeek API key
     * @param style  Cleanup style: "polished", "verbatim", or "chat"
     * @return Cleaned text
     * @throws IOException on network or API errors
     */
    fun cleanup(text: String, apiKey: String, style: String): String {
        val systemPrompt = when (style) {
            "verbatim" -> """You are a minimal text cleanup assistant. The user gives you raw speech-to-text output. Apply ONLY these changes:
- Remove filler words (um, uh, like, you know / äh, ähm, also, halt, sozusagen, quasi)
- Remove stutters and repeated words (e.g. "the the" → "the")
- Resolve mid-speech corrections: when the speaker backtracks (e.g. "tomorrow, no wait, Friday" → "Friday"), keep ONLY the final intended version
- Add punctuation and fix capitalization
- Fix obvious transcription errors (misheard words)
- Language: respond in the same language as the input. If the speaker mixes languages (e.g. German with English terms, or English with German words), preserve EXACTLY which words were said in which language. NEVER translate between languages.

STRICT RULES — you MUST follow these:
- NEVER change, rephrase, reorder, or add words beyond the rules above
- NEVER improve grammar or sentence structure
- NEVER remove hedge words like "ich denke", "vielleicht", "basically", "I think"
- NEVER remove greetings or interjections (hey, hi, ok, na ja, ach)
- NEVER add line breaks, paragraphs, lists, or any formatting
- NEVER add or remove meaning
- NEVER translate words from one language to another
- Output ONLY the cleaned text, no explanations"""
            "chat" -> """IMPORTANT: Your output language MUST match the input language. German input → German output. English input → English output. NEVER translate.

You are a text cleanup assistant. The user gives you raw speech-to-text output. Make it chat-ready:
- Remove all filler words and stutters
- Resolve mid-speech corrections: keep only the final version
- Make it concise — this is for messaging apps
- Keep it casual and natural
- Emojis are allowed where they fit naturally
- Language: respond in the SAME language as the input. If the speaker mixes languages, keep the mix — NEVER translate.
- Output ONLY the cleaned text, no explanations"""
            else -> """You are a text cleanup assistant. The user gives you raw speech-to-text output. Clean it up so it reads well:
- Remove filler words (um, uh, like, you know / äh, ähm, also, halt, sozusagen)
- Remove stutters and repeated words
- Resolve mid-speech corrections: keep ONLY the final intended version
- Fix grammar, punctuation, and capitalization
- Smooth sentence flow: fix run-on sentences, improve connectors, remove verbal padding ("du weißt schon", "you know what I mean", "und so weiter")
- You MAY lightly rephrase for clarity, but keep the speaker's voice
- Language: IMPORTANT — your output language MUST match the input language. German input → German output. English input → English output. If the speaker mixes languages, preserve EXACTLY which words were said in which language. NEVER translate between languages.

STRICT RULES:
- NEVER substitute words with different words that change the meaning. If the speaker said a specific word, keep that exact word
- NEVER add content, opinions, or information the speaker did not say
- NEVER restructure into lists, bullet points, or multiple paragraphs unless the speaker clearly enumerated items
- NEVER make it sound formal or academic — keep the speaker's natural register
- NEVER translate words from one language to another — keep code-switching as spoken
- Keep hedge words ("ich denke", "I think") — they reflect intent
- Output ONLY the cleaned text, no explanations"""
        }

        val url = URL("https://api.deepseek.com/chat/completions")
        val conn = url.openConnection() as HttpURLConnection

        try {
            conn.requestMethod = "POST"
            conn.doOutput = true
            conn.connectTimeout = 15_000
            conn.readTimeout = 30_000
            conn.setRequestProperty("Authorization", "Bearer $apiKey")
            conn.setRequestProperty("Content-Type", "application/json")

            val messages = JSONArray().apply {
                put(JSONObject().apply {
                    put("role", "system")
                    put("content", systemPrompt)
                })
                put(JSONObject().apply {
                    put("role", "user")
                    put("content", text)
                })
            }

            val requestBody = JSONObject().apply {
                put("model", "deepseek-chat")
                put("messages", messages)
                put("temperature", 0.3)
            }.toString().toByteArray(Charsets.UTF_8)

            conn.setRequestProperty("Content-Length", requestBody.size.toString())
            conn.outputStream.use { it.write(requestBody) }

            val responseCode = conn.responseCode
            if (responseCode != 200) {
                val errorBody = conn.errorStream?.bufferedReader()?.readText() ?: "unknown error"
                throw IOException("DeepSeek cleanup failed: HTTP $responseCode -- $errorBody")
            }

            val responseText = conn.inputStream.bufferedReader().readText()
            val json = JSONObject(responseText)
            return json
                .getJSONArray("choices")
                .getJSONObject(0)
                .getJSONObject("message")
                .getString("content")
                .trim()
        } finally {
            conn.disconnect()
        }
    }

    // --- Chunked cleanup ---

    private const val CHUNK_THRESHOLD = 800
    private const val CHUNK_TARGET_SIZE = 600
    private const val CLEANUP_TAG = "DiktaApi"

    /**
     * Splits text into chunks at sentence boundaries (`. `, `! `, `? `, or `\n`).
     * Each chunk targets ~CHUNK_TARGET_SIZE characters but does not break mid-sentence.
     * Mirrors the Rust `split_into_chunks` function in src-tauri/src/llm/mod.rs.
     *
     * @param text Input text to split
     * @return List of trimmed, non-empty chunks
     */
    fun splitIntoChunks(text: String): List<String> {
        val chunks = mutableListOf<String>()
        var start = 0

        while (start < text.length) {
            if (text.length - start <= CHUNK_TARGET_SIZE) {
                val tail = text.substring(start).trim()
                if (tail.isNotEmpty()) chunks.add(tail)
                break
            }

            // Search for a sentence boundary near the target size.
            // Search window: from (start + CHUNK_TARGET_SIZE/2) up to (start + CHUNK_TARGET_SIZE + 200).
            val searchEnd = (start + CHUNK_TARGET_SIZE + 200).coerceAtMost(text.length)
            var bestSplit: Int? = null

            var i = start + CHUNK_TARGET_SIZE / 2
            while (i < searchEnd) {
                val c = text[i]
                val next = if (i + 1 < text.length) text[i + 1] else '\u0000'

                if ((c == '.' || c == '!' || c == '?') && next == ' ') {
                    bestSplit = i + 1  // include the punctuation character
                    if (i >= start + CHUNK_TARGET_SIZE) break  // close enough to target
                } else if (c == '\n') {
                    bestSplit = i
                    if (i >= start + CHUNK_TARGET_SIZE) break
                }
                i++
            }

            val splitAt = bestSplit ?: (start + CHUNK_TARGET_SIZE).coerceAtMost(text.length)
            val chunk = text.substring(start, splitAt).trim()
            if (chunk.isNotEmpty()) chunks.add(chunk)

            // Advance past the split point, skipping leading whitespace.
            start = splitAt
            while (start < text.length && text[start].isWhitespace()) start++
        }

        return chunks
    }

    /**
     * Cleans up text using the DeepSeek API, with chunked parallel processing for long texts.
     *
     * - If text.length <= CHUNK_THRESHOLD: delegates to [cleanup] (single call).
     * - If text.length > CHUNK_THRESHOLD: splits into chunks via [splitIntoChunks] and
     *   processes all chunks in parallel using a fixed-size thread pool.
     *   Results are joined with "\n\n".
     *   If any chunk fails, falls back to a single [cleanup] call on the full text.
     *
     * @param text   Raw transcription text to clean up
     * @param apiKey DeepSeek API key
     * @param style  Cleanup style: "polished", "verbatim", or "chat"
     * @return Cleaned text
     * @throws IOException if both chunked and fallback calls fail
     */
    fun cleanupChunked(text: String, apiKey: String, style: String): String {
        if (text.length <= CHUNK_THRESHOLD) {
            return cleanup(text, apiKey, style)
        }

        val chunks = splitIntoChunks(text)
        if (chunks.size <= 1) {
            return cleanup(text, apiKey, style)
        }

        Log.i(CLEANUP_TAG, "[cleanupChunked] splitting ${text.length} chars into ${chunks.size} chunks")

        val executor = Executors.newFixedThreadPool(4)
        try {
            val futures = chunks.map { chunk ->
                executor.submit(Callable { cleanup(chunk, apiKey, style) })
            }

            // Collect results -- if any Future throws, we fall through to the catch block.
            val results = try {
                futures.map { it.get() }
            } catch (e: Exception) {
                Log.w(CLEANUP_TAG, "[cleanupChunked] a chunk failed, falling back to single call", e)
                return cleanup(text, apiKey, style)
            }

            return results.joinToString("\n\n")
        } finally {
            executor.shutdown()
        }
    }

    // --- Helpers ---

    private fun buildMultipartBody(boundary: String, wavBytes: ByteArray, language: String): ByteArray {
        val out = ByteArrayOutputStream()
        val writer = PrintWriter(OutputStreamWriter(out, Charsets.UTF_8), true)

        // model field
        writer.append("--$boundary\r\n")
        writer.append("Content-Disposition: form-data; name=\"model\"\r\n\r\n")
        writer.append("whisper-large-v3-turbo\r\n")
        writer.flush()

        // response_format field
        writer.append("--$boundary\r\n")
        writer.append("Content-Disposition: form-data; name=\"response_format\"\r\n\r\n")
        writer.append("json\r\n")
        writer.flush()

        // language field (skip if empty -- Whisper auto-detects)
        if (language.isNotBlank()) {
            writer.append("--$boundary\r\n")
            writer.append("Content-Disposition: form-data; name=\"language\"\r\n\r\n")
            writer.append("$language\r\n")
            writer.flush()
        }

        // audio file field
        writer.append("--$boundary\r\n")
        writer.append("Content-Disposition: form-data; name=\"file\"; filename=\"audio.wav\"\r\n")
        writer.append("Content-Type: audio/wav\r\n\r\n")
        writer.flush()

        out.write(wavBytes)

        writer.append("\r\n--$boundary--\r\n")
        writer.flush()

        return out.toByteArray()
    }
}

/**
 * Encodes raw PCM short samples to a WAV byte array (16kHz, mono, 16-bit).
 */
fun encodeWav(pcmData: ShortArray, sampleRate: Int = 16000): ByteArray {
    val byteRate = sampleRate * 2  // 16-bit mono = 2 bytes per sample
    val dataSize = pcmData.size * 2
    val totalSize = 44 + dataSize

    val buffer = java.nio.ByteBuffer.allocate(totalSize).order(java.nio.ByteOrder.LITTLE_ENDIAN)

    // RIFF chunk
    buffer.put("RIFF".toByteArray(Charsets.US_ASCII))
    buffer.putInt(totalSize - 8)
    buffer.put("WAVE".toByteArray(Charsets.US_ASCII))

    // fmt sub-chunk
    buffer.put("fmt ".toByteArray(Charsets.US_ASCII))
    buffer.putInt(16)            // sub-chunk size (PCM)
    buffer.putShort(1)           // audio format: PCM
    buffer.putShort(1)           // channels: mono
    buffer.putInt(sampleRate)    // sample rate
    buffer.putInt(byteRate)      // byte rate
    buffer.putShort(2)           // block align (channels * bits/8)
    buffer.putShort(16)          // bits per sample

    // data sub-chunk
    buffer.put("data".toByteArray(Charsets.US_ASCII))
    buffer.putInt(dataSize)
    for (sample in pcmData) {
        buffer.putShort(sample)
    }

    return buffer.array()
}
