use rodio::OutputStreamHandle;

pub struct AudioData {
    pub stream: rodio::OutputStream,
    pub stream_handle: OutputStreamHandle,
}


impl AudioData {
    pub fn new() -> anyhow::Result<AudioData> {
        let (stream, handle) = rodio::OutputStream::try_default()?;
        Ok(Self {
            stream,
            stream_handle: handle,
        })
    }
}


impl AudioData {}