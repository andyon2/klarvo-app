package com.dikta.voice

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.os.Build
import org.json.JSONObject
import org.json.JSONArray
import java.io.*
import java.net.HttpURLConnection
import java.net.URL

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
        val cleanupStyle: String
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

            if (groqKey.isBlank() && deepseekKey.isBlank()) null
            else Config(groqKey, deepseekKey, language, cleanupStyle)
        } catch (e: Exception) {
            null
        }
    }

    /**
     * Saves a transcription entry to the SQLite history database.
     * Uses the same schema as the Rust/Tauri desktop app so history is shared.
     *
     * @param context   Android context (used to resolve the DB path)
     * @param finalText Cleaned/final text shown to the user
     * @param rawText   Raw transcript before LLM cleanup
     * @param style     Cleanup style (e.g. "polished", "verbatim", "chat")
     * @param language  Language code or empty string for auto-detect
     */
    fun saveToHistory(
        context: Context,
        finalText: String,
        rawText: String,
        style: String,
        language: String
    ) {
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
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                )
                """.trimIndent()
            )
            val stmt = db.compileStatement(
                "INSERT INTO history (text, raw_text, style, language, is_note, app_name) VALUES (?, ?, ?, ?, 0, NULL)"
            )
            stmt.bindString(1, finalText)
            stmt.bindString(2, rawText)
            stmt.bindString(3, style)
            stmt.bindString(4, language)
            stmt.executeInsert()
        } catch (_: Exception) {
            // History saving is best-effort; never crash the main flow.
        } finally {
            try { db?.close() } catch (_: Exception) {}
        }
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
            "verbatim" -> "You are a dictation cleanup assistant. Remove filler words (um, uh, like) but keep the speaker's exact words. Output ONLY the cleaned text, nothing else."
            "chat" -> "You are a dictation cleanup assistant. Make the text short and casual, like a chat message. Output ONLY the cleaned text, nothing else."
            else -> "You are a dictation cleanup assistant. Fix grammar, remove filler words, improve clarity. Keep the meaning intact. Output ONLY the cleaned text, nothing else."
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
