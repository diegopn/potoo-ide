#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AudioCommand {
    Load { path: String, cache_by_path: bool },
    Release { handle: i64 },
    Play { handle: i64, repeat: bool },
    Pause { handle: i64 },
    Resume { handle: i64 },
    IsPlaying { handle: i64 },
    IsPaused { handle: i64 },
    Stop { handle: i64 },
    Duration { handle: i64 },
    Position { handle: i64 },
    Seek { handle: i64, milliseconds: i64 },
    SetPlaybackVolume { handle: i64, percent: i64 },
    SetMasterVolume { percent: i64 },
    GetMasterVolume,
    GetPlaybackVolume { handle: i64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AudioResponse {
    Unit,
    Handle(i64),
    Integer(i64),
    Logical(bool),
}
