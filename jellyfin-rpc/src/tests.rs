use crate::application::template::TemplateRenderer;
use crate::domain::{
    Blacklist, DeviceInfo, DisplayFormat, Library, MediaType, NowPlayingItem, PlayState, Session,
};
use crate::{RpcBuilder, ServerConfig};

fn session(item: NowPlayingItem) -> Session {
    Session {
        item_id: item.id.clone(),
        now_playing_item: item,
        play_state: PlayState::default(),
        device: DeviceInfo {
            client: Some("Jellyfin Android".to_string()),
            device_name: Some("Pixel 7".to_string()),
            remote_end_point: None,
        },
        source_name: "local".to_string(),
        user_name: "akmal".to_string(),
    }
}

#[test]
fn build_without_servers_fails() {
    let builder = RpcBuilder::new();

    assert!(builder.build().is_err(), "built a service with no servers");
}

#[test]
fn build_without_username_fails() {
    let mut builder = RpcBuilder::new();
    builder.add_server(ServerConfig::new("vps", "https://example.com", "a1b2c3d4"));

    assert!(builder.build().is_err(), "built a service with no username");
}

#[test]
fn build_with_invalid_url_fails() {
    let mut builder = RpcBuilder::new();
    builder
        .username("test")
        .add_server(ServerConfig::new("vps", "url_without_base.com", "a1b2c3d4"));

    assert!(builder.build().is_err(), "built a service with a bad url");
}

#[test]
fn per_server_username_satisfies_the_requirement() {
    let mut server = ServerConfig::new("local", "http://localhost:8096", "a1b2c3d4");
    server.usernames = vec!["akmal".to_string()];

    let mut builder = RpcBuilder::new();
    builder.add_server(server);

    // No global username, but the server carries its own, so the config is complete.
    assert!(builder.build().is_ok());
}

#[test]
fn template_expands_session_context() {
    let mut item = NowPlayingItem {
        name: "Dune".to_string(),
        media_type: MediaType::Movie,
        id: "1".to_string(),
        production_year: Some(2021),
        ..Default::default()
    };
    item.run_time_ticks = Some(155 * 60 * 10_000_000);

    let session = session(item);
    let renderer = TemplateRenderer::new(&session, " • ");

    assert_eq!(
        renderer.render_movie("{title} {sep} {year} {sep} {duration-minutes}"),
        "Dune • 2021 • 155 Minutes"
    );
    assert_eq!(renderer.render_movie("{device}"), "Pixel 7");
    assert_eq!(renderer.render_movie("{server}"), "local");
}

#[test]
fn template_drops_separators_left_by_empty_placeholders() {
    let item = NowPlayingItem {
        name: "Dune".to_string(),
        media_type: MediaType::Movie,
        id: "1".to_string(),
        ..Default::default()
    };

    let session = session(item);
    let renderer = TemplateRenderer::new(&session, " • ");

    // `{year}` and `{genres}` are empty here.
    assert_eq!(
        renderer.render_movie("{year} {sep} {title} {sep} {genres}"),
        "Dune"
    );
}

#[test]
fn display_format_from_legacy_list() {
    let format = DisplayFormat::from(vec!["genres".to_string(), "year".to_string()]);

    assert_eq!(
        format.state_text.as_deref(),
        Some("{__default}{genres} {sep} {year}")
    );
}

#[test]
fn blacklist_blocks_by_media_type_and_library_path() {
    let blacklist = Blacklist::new(vec![MediaType::Music], vec!["Anime".to_string()]);

    let music = NowPlayingItem {
        media_type: MediaType::Music,
        ..Default::default()
    };
    assert!(blacklist.blocks(&music, &[]));

    let libraries = vec![Library {
        name: Some("Anime".to_string()),
        locations: vec!["D:/Media/Anime".to_string()],
    }];

    let episode = NowPlayingItem {
        media_type: MediaType::Episode,
        path: Some("D:/Media/Anime/Frieren/S01E01.mkv".to_string()),
        ..Default::default()
    };
    assert!(blacklist.blocks(&episode, &libraries));

    let movie = NowPlayingItem {
        media_type: MediaType::Movie,
        path: Some("D:/Media/Movies/Dune.mkv".to_string()),
        ..Default::default()
    };
    assert!(!blacklist.blocks(&movie, &libraries));
}

#[test]
fn blacklist_resolves_only_named_libraries() {
    let blacklist = Blacklist::new(Vec::new(), vec!["Anime".to_string()]);

    let libraries = [
        Library {
            name: Some("Anime".to_string()),
            locations: vec!["D:/Media/Anime".to_string()],
        },
        Library {
            name: Some("Movies".to_string()),
            locations: vec!["D:/Media/Movies".to_string()],
        },
    ];

    let resolved = blacklist.resolve(libraries.iter());
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].name.as_deref(), Some("Anime"));
}
