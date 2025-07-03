use super::{ParseFrom, ProcessedResult};
use crate::common::{LyricsID, SongTrackingUrl};
use crate::nav_consts::{DESCRIPTION, DESCRIPTION_SHELF, RUN_TEXT, SECTION_LIST_ITEM};
use crate::query::song::{GetLyricsIDQuery, GetSongTrackingUrlQuery};
use crate::query::GetLyricsQuery;
use const_format::concatcp;
use json_crawler::{JsonCrawler, JsonCrawlerOwned};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Lyrics {
    pub lyrics: String,
    pub source: String,
}

impl<'a> ParseFrom<GetSongTrackingUrlQuery<'a>> for SongTrackingUrl<'static> {
    fn parse_from(p: super::ProcessedResult<GetSongTrackingUrlQuery<'a>>) -> crate::Result<Self> {
        let mut crawler = JsonCrawlerOwned::from(p);
        crawler
            .take_value_pointer("/playbackTracking/videostatsPlaybackUrl/baseUrl")
            .map_err(Into::into)
    }
}

impl<'a> ParseFrom<GetLyricsIDQuery<'a>> for LyricsID<'static> {
    fn parse_from(p: ProcessedResult<GetLyricsIDQuery<'a>>) -> crate::Result<Self> {
        let mut json_crawler: JsonCrawlerOwned = p.into();
        let lyrics_id_path = "/contents/singleColumnMusicWatchNextResultsRenderer/tabbedRenderer/watchNextTabbedResultsRenderer/tabs/1/tabRenderer/endpoint/browseEndpoint/browseId";
        json_crawler
            .take_value_pointer(lyrics_id_path)
            .map_err(Into::into)
    }
}

impl<'a> ParseFrom<GetLyricsQuery<'a>> for Lyrics {
    fn parse_from(p: ProcessedResult<GetLyricsQuery<'a>>) -> crate::Result<Self> {
        let json_crawler: JsonCrawlerOwned = p.into();
        // TODO: May also get a "Lyrics not available" message at
        // /contents/messageRenderer/text/runs/0/text
        let mut description_shelf = json_crawler.navigate_pointer(concatcp!(
            "/contents",
            SECTION_LIST_ITEM,
            DESCRIPTION_SHELF
        ))?;
        Ok(Lyrics {
            lyrics: description_shelf.take_value_pointer(DESCRIPTION)?,
            source: description_shelf.take_value_pointer(concatcp!("/footer", RUN_TEXT))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::auth::BrowserToken;
    use crate::common::{LyricsID, SongTrackingUrl, VideoID, YoutubeID};
    use crate::parse::song::Lyrics;
    use crate::process_json;
    use crate::query::song::GetSongTrackingUrlQuery;
    use crate::query::{GetLyricsIDQuery, GetLyricsQuery};

    #[tokio::test]
    async fn test_get_song_tracking_url_query() {
        let output = SongTrackingUrl::from_raw("https://s.youtube.com/api/stats/playback?cl=655300395&docid=FZ8BxMU3BYc&ei=JSimZqHaNeyB9fwP9oqh0Ak&fexp=&ns=yt&plid=AAYeTNocW-liNkl6&el=detailpage&len=193&of=URbTjA0hNUiM-oZxeU_KzQ&osid=AAAAAYfxXtM%3AAOeUNAZhCDiglWHfELd4I0ksz0dyuGtLVg&uga=m32&vm=CAMQARgBOjJBSHFpSlRJMDQteFk3b0Z2MUZXblN3NTlza3ZKcEhkcXpWeVhhMXl4RGQyZXVFR2twZ2JiQU9BckJGdG4zbDdCcElKTGJHNkt3dlJVX2ZzZGdKMndGR1ZZdk92MVItWWYtUTBOYmdFQnYxd3J6cGJBNzdrZUJXMlQ0QWR4MVo4S1Rza1JTM0hvWGRTd2llYk5xZFd6Nne4AQE");
        parse_test_value!(
            "./test_json/get_song_tracking_url_20240728.json",
            output,
            GetSongTrackingUrlQuery::new(VideoID::from_raw("")).unwrap(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_lyrics_id() {
        parse_test_value!(
            "./test_json/get_watch_playlist_20250630.json",
            LyricsID::from_raw("MPLYt_dcYZhAh5urI-1"),
            GetLyricsIDQuery::new(VideoID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_lyrics_query() {
        // Intro - Notorious BIG - Ready To Die
        let path = std::path::Path::new("./test_json/get_lyrics_20231219.json");
        let file = tokio::fs::read_to_string(path)
            .await
            .expect("Expect file read to pass during tests");
        // Blank query has no bearing on function
        let query = GetLyricsQuery::new(LyricsID::from_raw(""));
        let output = process_json::<_, BrowserToken>(file, query).unwrap();
        assert_eq!(
                output,
                Lyrics {
                    lyrics: "Push \r\nCome on, she almost there push, come on\r\nCome on, come on, push, it's almost there \r\nOne more time, come one\r\nCome on, push, baby, one more time \r\nHarder, harder, push it harder \r\nPush, push, come on \r\nOne more time, here it goes \r\nI see the head\r\nYeah, come on\r\nYeah, yeah\r\nYou did it, baby, yeah\r\n\r\nBut if you lose, don't ask no questions why\r\nThe only game you know is do or die\r\nAh-ha-ha\r\nHard to understand what a hell of a man\r\n\r\nHip hop the hippie the hippie\r\nTp the hip hop and you don't stop \r\nRock it out, baby bubba, to the boogie, the bang-bang\r\nThe boogie to the boogie that be\r\nNow what you hear is not a test, I'm rappin', to the beat \r\n\r\nGoddamn it, Voletta, what the fuck are you doin'?\r\nYou can't control that goddamn boy? (What?)\r\nI just saw Mr. Johnson, he told me he caught the motherfucking boy shoplifting \r\nWhat the fuck are you doing? (Kiss my black ass, motherfucker)\r\nYou can't control that god-, I don't know what the fuck to do with that boy\r\n(What the fuck do you want me to do?)\r\nIf if you can't fucking control that boy, I'ma send him\r\n(All you fucking do is bitch at me)\r\nBitch, bitch, I'ma send his motherfuckin' ass to a group home goddamnit, what?\r\nI'll smack the shit outta you bitch, what, what the fuck?\r\n(Kiss my black ass, motherfucker)\r\nYou're fuckin' up\r\n(Comin' in here smelling like sour socks you, dumb motherfucker) \r\n\r\nWhen I'm bustin' up a party I feel no guilt\r\nGizmo's cuttin' up for thee \r\nSuckers that's down with nei-\r\n\r\nWhat, nigga, you wanna rob them motherfuckin' trains, you crazy? \r\nYes, yes, motherfucker, motherfuckin' right, nigga, yes \r\nNigga, what the fuck, nigga? We gonna get-\r\nNigga, it's eighty-seven nigga, is you dead broke? \r\nYeah, nigga, but, but\r\nMotherfucker, is you broke, motherfucker? \r\nWe need to get some motherfuckin' paper, nigga \r\nNigga it's a train, ain't nobody never robbed no motherfuckin' train \r\nJust listen, man, is your mother givin' you money, nigga? \r\nMy moms don't give me shit nigga, it's time to get paid, nigga \r\nIs you with me? Motherfucker, is you with me? \r\nYeah, I'm with you, nigga, come on \r\nAlright then, nigga, lets make it happen then \r\nAll you motherfuckers get on the fuckin' floor \r\nGet on the motherfuckin' floor\r\nChill, give me all your motherfuckin' money \r\nAnd don't move, nigga\r\nI want the fuckin' jewelry \r\nGive me every fuckin' thing \r\nNigga, I'd shut the fuck up or I'ma blow your motherfuckin' brains out \r\nShut the fuck up, bitch, give me your fuckin' money, motherfucker\r\nFuck you, bitch, get up off that shit \r\nWhat the fuck you holdin' on to that shit for, bitch? \r\n\r\nI get money, money I got\r\nStunts call me honey if they feel real hot\r\n\r\nOpen C-74, Smalls \r\nMr. Smalls, let me walk you to the door \r\nSo how does it feel leavin' us? \r\nCome on, man, what kind of fuckin' question is that, man? \r\nTryin' to get the fuck up out this joint, dog \r\nYeah, yeah, you'll be back \r\nYou niggas always are \r\nGo ahead, man, what the fuck is you hollerin' about? \r\nYou won't see me up in this motherfucker no more \r\nWe'll see \r\nI got big plans nigga, big plans, hahaha".to_string(),
                    source: "Source: LyricFind".to_string()
                }
            );
    }
}
