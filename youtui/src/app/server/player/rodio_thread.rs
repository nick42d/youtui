fn try_decode(
    song: Arc<InMemSong>,
    song_id: ListSongID,
    tx: mpsc::Sender<PlaySongResponse>,
) -> std::result::Result<
    PeriodicAccess<
        TrackPosition<Decoder<Cursor<DroppableSong>>>,
        impl FnMut(&mut TrackPosition<Decoder<Cursor<DroppableSong>>>),
    >,
    DecoderError,
> {
    // DUPLICATE FROM PLAYSONG
    let sp = DroppableSong {
        song,
        song_id,
        channel: tx.clone(),
    };
    let cur = std::io::Cursor::new(sp);
    rodio::Decoder::new(cur).map(move |s| {
        s.track_position()
            .periodic_access(PROGRESS_UPDATE_DELAY, move |s| {
                blocking_send_or_error(&tx, PlaySongResponse::ProgressUpdate(s.get_pos()));
            })
    })
}
