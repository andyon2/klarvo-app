package com.klarvo.voice

import android.content.Context
import android.util.Log
import java.io.File
import java.io.FileWriter
import java.io.IOException
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/**
 * Dual-sink logger: writes to Logcat AND to a rotating file at {dataDir}/logs/klarvo.log.
 *
 * Call KlarvoLogger.init(context) once in KlarvoOverlayService.onCreate() before use.
 * All methods are thread-safe (synchronized on the companion object's lock).
 *
 * Rotation: when the active file exceeds MAX_FILE_SIZE_BYTES it is renamed to klarvo.1.log,
 * older files shift up, and the oldest beyond MAX_FILES is deleted.
 */
object KlarvoLogger {

    private const val MAX_FILE_SIZE_BYTES = 2 * 1024 * 1024  // 2 MB
    private const val MAX_FILES = 5

    private val lock = Any()
    private val timestampFmt = SimpleDateFormat("yyyy-MM-dd HH:mm:ss.SSS", Locale.US)

    @Volatile
    private var logFile: File? = null

    // ---- Init ----

    /**
     * Must be called once before any log call.
     * Safe to call multiple times (idempotent).
     */
    fun init(context: Context) {
        synchronized(lock) {
            if (logFile != null) return
            val logsDir = File(context.dataDir, "logs")
            logsDir.mkdirs()
            logFile = File(logsDir, "klarvo.log")
        }
    }

    // ---- Public API ----

    fun d(tag: String, msg: String) {
        Log.d(tag, msg)
        write("D", tag, msg, null)
    }

    fun i(tag: String, msg: String) {
        Log.i(tag, msg)
        write("I", tag, msg, null)
    }

    fun w(tag: String, msg: String) {
        Log.w(tag, msg)
        write("W", tag, msg, null)
    }

    fun w(tag: String, msg: String, throwable: Throwable) {
        Log.w(tag, msg, throwable)
        write("W", tag, msg, throwable)
    }

    fun e(tag: String, msg: String) {
        Log.e(tag, msg)
        write("E", tag, msg, null)
    }

    fun e(tag: String, msg: String, throwable: Throwable) {
        Log.e(tag, msg, throwable)
        write("E", tag, msg, throwable)
    }

    // ---- Internal ----

    private fun write(level: String, tag: String, msg: String, throwable: Throwable?) {
        val target = logFile ?: return  // not initialised -- skip file write
        val timestamp = timestampFmt.format(Date())
        val line = buildString {
            append(timestamp)
            append(' ')
            append(level)
            append('/')
            append(tag)
            append(": ")
            append(msg)
            if (throwable != null) {
                append('\n')
                append(throwable.stackTraceToString())
            }
            append('\n')
        }
        synchronized(lock) {
            try {
                rotateIfNeeded(target)
                FileWriter(target, /* append */ true).use { it.write(line) }
            } catch (_: IOException) {
                // Silently ignore file-write errors -- Logcat is always the primary sink.
            }
        }
    }

    /**
     * If the current log file exceeds MAX_FILE_SIZE_BYTES, rotate:
     *   klarvo.4.log -> deleted
     *   klarvo.3.log -> klarvo.4.log
     *   ...
     *   klarvo.log   -> klarvo.1.log
     */
    private fun rotateIfNeeded(active: File) {
        if (!active.exists() || active.length() < MAX_FILE_SIZE_BYTES) return

        val dir = active.parentFile ?: return

        // Delete the oldest file if it exists
        val oldest = File(dir, "klarvo.${MAX_FILES - 1}.log")
        oldest.delete()

        // Shift files up: klarvo.(n-1).log -> klarvo.n.log
        for (i in (MAX_FILES - 2) downTo 1) {
            val from = File(dir, "klarvo.$i.log")
            val to   = File(dir, "klarvo.${i + 1}.log")
            if (from.exists()) from.renameTo(to)
        }

        // Rotate active -> klarvo.1.log
        active.renameTo(File(dir, "klarvo.1.log"))
    }
}
