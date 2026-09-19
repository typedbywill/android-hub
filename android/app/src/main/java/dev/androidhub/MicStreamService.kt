package dev.androidhub

import android.app.*
import android.content.Intent
import android.content.pm.ServiceInfo
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import java.io.BufferedOutputStream
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.concurrent.thread

/** USB-only server: adb forward exposes this localhost socket to the desktop. */
class MicStreamService : Service() {
    companion object { const val PORT = 61394; const val RATE = 48000; const val CHANNELS = 1; const val MAX_FRAME = 3840; private const val CHANNEL = "android-hub-mic" }
    private val running = AtomicBoolean(false)
    private var server: ServerSocket? = null
    private var worker: Thread? = null
    override fun onBind(intent: Intent?): IBinder? = null
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int { if (!running.getAndSet(true)) { startForegroundService(); worker = thread(name="AndroidHubMic") { serve() } }; return START_NOT_STICKY }
    private fun startForegroundService() {
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(NotificationChannel(CHANNEL, "Android Hub microphone", NotificationManager.IMPORTANCE_LOW))
        val notification = NotificationCompat.Builder(this, CHANNEL).setSmallIcon(android.R.drawable.ic_btn_speak_now).setContentTitle("Android Hub microphone active").setContentText("Sharing microphone over USB").setOngoing(true).build()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) startForeground(1, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE) else startForeground(1, notification)
    }
    private fun serve() {
        try { ServerSocket(PORT, 1).use { socket -> server = socket; while (running.get()) { socket.accept().use { client -> streamAudio(client) } } } } catch (_: Exception) { } finally { server = null; running.set(false); stopSelf() }
    }
    private fun streamAudio(client: Socket) {
        val min = AudioRecord.getMinBufferSize(RATE, AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT)
        val record = AudioRecord(MediaRecorder.AudioSource.VOICE_COMMUNICATION, RATE, AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT, maxOf(min, MAX_FRAME * 4))
        val buffer = ByteArray(MAX_FRAME)
        try { BufferedOutputStream(client.getOutputStream()).use { output -> output.write(byteArrayOf('A'.code.toByte(), 'H'.code.toByte(), 'U'.code.toByte(), 'B'.code.toByte(), 1, 1, 1, 0)); output.write(byteArrayOf(0, 0, 0xBB.toByte(), 0x80.toByte())); output.flush(); record.startRecording(); while (running.get()) { val size = record.read(buffer, 0, buffer.size); if (size > 0) { output.write(byteArrayOf(0, 0, (size shr 8).toByte(), size.toByte())); output.write(buffer, 0, size); output.flush() } } } } finally { record.stop(); record.release() }
    }
    override fun onDestroy() { running.set(false); server?.close(); worker?.interrupt(); super.onDestroy() }
}

