package dev.androidhub

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat

class MainActivity : AppCompatActivity() {
    private lateinit var status: TextView
    private val requestAudio = registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        status.text = if (granted) "Microphone permission granted. Connect from the Linux terminal." else "Microphone permission is required to use Android as a microphone."
    }
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val layout = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(48, 64, 48, 48) }
        layout.addView(TextView(this).apply { text = "Android Hub\n\nThis device can provide its microphone over USB debugging. Camera forwarding is managed by the Linux client through scrcpy."; textSize = 19f })
        status = TextView(this).apply { textSize = 16f; setPadding(0, 36, 0, 24) }
        layout.addView(status)
        layout.addView(Button(this).apply { text = "Grant microphone permission"; setOnClickListener { requestPermission() } })
        layout.addView(Button(this).apply { text = "Stop microphone sharing"; setOnClickListener { stopService(Intent(this@MainActivity, MicStreamService::class.java)); status.text = "Microphone sharing stopped." } })
        setContentView(layout); requestPermission()
    }
    private fun requestPermission() {
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED) status.text = "Ready. Run android-hub start --mic on Linux." else requestAudio.launch(Manifest.permission.RECORD_AUDIO)
    }
}

