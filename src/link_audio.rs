use std::{ffi::CString, marker::PhantomData, os::raw::c_char};

use crate::{AblLink, SessionState, rust_bindings::*};

/// A list of Link Audio channels.
pub struct LinkAudioChannelList {
    pub(crate) list: abl_link_audio_channel_list,
}

impl LinkAudioChannelList {
    pub fn len(&self) -> usize {
        self.list.count
    }
}

impl Drop for LinkAudioChannelList {
    fn drop(&mut self) {
        unsafe { abl_link_audio_free_channel_list(self.list) };
    }
}

pub struct LinkAudioSink {
    pub(crate) sink: abl_link_audio_sink,
}

impl LinkAudioSink {
    ///  Construct a Link Audio sink to announce an audio channel.
    pub fn new(link: &AblLink, name: String, max_num_samples: usize) -> Option<Self> {
        let Ok(c_string) = CString::new(name) else {
            return None;
        };
        Some(Self {
            sink: unsafe {
                abl_link_audio_sink_create(link.link, c_string.as_ptr(), max_num_samples)
            },
        })
    }

    /// Set the name of a Link Audio sink.
    ///
    /// Thread-safe: yes
    ///
    /// Realtime-safe: no
    pub fn set_name(&self, name: String) -> bool {
        let Ok(c_string) = CString::new(name) else {
            return false;
        };
        unsafe { abl_link_audio_sink_set_name(self.sink, c_string.as_ptr()) };
        true
    }

    /// Get the name of a Link Audio sink.
    pub fn get_name(&self) -> String {
        let mut buffer = vec![0 as c_char; 256];
        let written =
            unsafe { abl_link_audio_sink_name(self.sink, buffer.as_mut_ptr(), buffer.len()) };
        let bytes: Vec<u8> = buffer[..written].iter().map(|&c| c as u8).collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Get the current maximum number of samples a buffer handle can hold.
    ///
    /// Thread-safe: yes
    ///
    /// Realtime-safe: yes
    pub fn max_num_samples(&self) -> usize {
        unsafe { abl_link_audio_sink_max_num_samples(self.sink) }
    }

    /// Request a maximum buffer size for future buffers.
    ///
    /// Thread-safe: yes
    ///
    /// Realtime-safe: yes
    pub fn request_max_num_samples(&self, max_num_samples: usize) {
        unsafe {
            abl_link_audio_sink_request_max_num_samples(self.sink, max_num_samples);
        }
    }

    /// Retain a buffer for writing audio samples. Only one buffer can be retained at a time.
    ///
    /// Thread-safe: no
    ///
    /// Realtime-safe: yes
    pub fn retain_buffer<'a>(&'a self) -> LinkAudioSinkBufferHandle<'a> {
        LinkAudioSinkBufferHandle {
            buffer: unsafe { abl_link_audio_sink_retain_buffer(self.sink) },
            _p: PhantomData,
        }
    }
}

impl Drop for LinkAudioSink {
    fn drop(&mut self) {
        unsafe { abl_link_audio_sink_destroy(self.sink) };
    }
}

///  Identifier for Link Audio channels/peers/sessions.
pub struct LinkAudioChannelId {
    pub(crate) id: abl_link_audio_channel_id,
}

impl PartialEq for LinkAudioChannelId {
    fn eq(&self, other: &Self) -> bool {
        self.id.bytes == other.id.bytes
    }
}

pub struct LinkAudioSessionId {
    pub(crate) id: abl_link_audio_session_id,
}

impl PartialEq for LinkAudioSessionId {
    fn eq(&self, other: &Self) -> bool {
        self.id.bytes == other.id.bytes
    }
}

/// Handle to a buffer for writing audio samples.
pub struct LinkAudioSinkBufferHandle<'a> {
    pub(crate) buffer: abl_link_audio_sink_buffer_handle,
    _p: PhantomData<&'a ()>,
}

fn f32_to_i16(sample: f32) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    (clamped * 32767.0) as i16
}

fn i16_to_f32(sample: i16) -> f32 {
    sample as f32 / 32768.0
}

impl<'a> LinkAudioSinkBufferHandle<'a> {
    /// Check if a buffer handle is valid.
    ///
    /// Thread-safe: no
    ///
    /// Realtime-safe: yes
    pub fn is_valid(&self) -> bool {
        unsafe { abl_link_audio_sink_buffer_is_valid(&self.buffer as *const _) }
    }

    /// Writes a single u16 audio sample to the given index.
    ///
    /// Panics if the index is larger than the maximum number of samples.
    pub fn write_sample(&mut self, index: usize, sample: i16) {
        assert!(
            index < self.buffer.max_num_samples,
            "Audio sink max length is {}, tried to write sample {}.",
            self.buffer.max_num_samples,
            index
        );
        unsafe { *self.buffer.samples.add(index) = sample };
    }

    /// Writes a single f32 audio sample to the given index.
    ///
    /// Panics if the index is larger than the maximum number of samples.
    pub fn write_sample_f32(&mut self, index: usize, sample: f32) {
        self.write_sample(index, f32_to_i16(sample));
    }

    pub fn max_num_samples(&self) -> usize {
        self.buffer.max_num_samples
    }

    /// Commit a buffer after writing samples. num_frames * num_channels must not exceed max_num_samples. The session state, quantum, and beats_at_buffer_begin must match those used for rendering.
    ///
    /// Thread-safe: no
    ///
    /// Realtime-safe: yes
    pub fn commit(
        mut self,
        session_state: SessionState,
        beats_at_buffer_begin: f64,
        quantum: f64,
        num_frames: usize,
        num_channels: usize,
        sample_rate: u32,
    ) -> bool {
        unsafe {
            abl_link_audio_sink_buffer_commit(
                &mut self.buffer as *mut _,
                session_state.session_state,
                beats_at_buffer_begin,
                quantum,
                num_frames,
                num_channels,
                sample_rate,
            )
        }
    }
}

impl<'a> Drop for LinkAudioSinkBufferHandle<'a> {
    fn drop(&mut self) {
        unsafe { abl_link_audio_sink_buffer_release(&mut self.buffer as *mut _) };
    }
}

pub struct LinkAudioSource {
    pub(crate) source: abl_link_audio_source,
}

impl LinkAudioSource {
    /// Get the channel ID of the corresponding source.
    ///
    /// Thread-safe: yes
    ///
    /// Realtime-safe: yes
    pub fn get_channel_id(&self) -> LinkAudioChannelId {
        LinkAudioChannelId {
            id: unsafe { abl_link_audio_source_id(self.source) },
        }
    }
}

impl Drop for LinkAudioSource {
    fn drop(&mut self) {
        unsafe { abl_link_audio_source_destroy(self.source) };
    }
}

pub struct LinkAudioSourceBuffer<'a> {
    pub(crate) buffer: &'a abl_link_audio_source_buffer,
}

impl<'a> LinkAudioSourceBuffer<'a> {
    pub fn num_channels(&self) -> usize {
        self.buffer.info.num_channels
    }

    pub fn num_frames(&self) -> usize {
        self.buffer.info.num_frames
    }

    pub fn sample_rate(&self) -> u32 {
        self.buffer.info.sample_rate
    }

    pub fn count(&self) -> u64 {
        self.buffer.info.count
    }

    pub fn session_beat_time(&self) -> f64 {
        self.buffer.info.session_beat_time
    }

    pub fn tempo(&self) -> f64 {
        self.buffer.info.tempo
    }

    pub fn session_id(&self) -> LinkAudioSessionId {
        LinkAudioSessionId {
            id: self.buffer.info.session_id,
        }
    }

    pub fn samples(&self) -> &[i16] {
        unsafe { std::slice::from_raw_parts(self.buffer.samples, self.buffer.info.num_frames) }
    }

    pub fn samples_f32(&self) -> impl Iterator<Item = f32> {
        self.samples().iter().map(|v| i16_to_f32(*v))
    }

    /// Map the beat time at the begin of the buffer to the local Link session state.
    ///
    /// Returns true if the buffer originates from the same Link Session and out_beats is set.
    pub fn begin_beats(&self, session_state: SessionState, quantum: f64) -> Option<f64> {
        unsafe {
            let mut out_beats = 0.0f64;
            if abl_link_audio_source_buffer_info_begin_beats(
                &self.buffer.info as *const abl_link_audio_source_buffer_info,
                session_state.session_state,
                quantum,
                &mut out_beats as *mut f64,
            ) {
                Some(out_beats)
            } else {
                None
            }
        }
    }

    /// Map the beat time at the end of the buffer to the local Link session state.
    ///
    /// Returns true if the buffer originates from the same Link Session and out_beats is set.
    pub fn end_beats(&self, session_state: SessionState, quantum: f64) -> Option<f64> {
        unsafe {
            let mut out_beats = 0.0f64;
            if abl_link_audio_source_buffer_info_end_beats(
                &self.buffer.info as *const abl_link_audio_source_buffer_info,
                session_state.session_state,
                quantum,
                &mut out_beats as *mut f64,
            ) {
                Some(out_beats)
            } else {
                None
            }
        }
    }
}
