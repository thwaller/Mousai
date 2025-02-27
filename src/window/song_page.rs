use gettextrs::gettext;
use gtk::{
    gdk, gio,
    glib::{self, clone, closure_local},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::unsync::OnceCell;

use std::cell::RefCell;

use super::{
    album_cover::AlbumCover,
    external_link_tile::ExternalLinkTile,
    information_row::InformationRow,
    playback_button::{PlaybackButton, PlaybackButtonMode},
};
use crate::{
    model::{ExternalLinkWrapper, Song},
    player::{Player, PlayerState},
    utils, Application,
};

mod imp {
    use super::*;
    use glib::{subclass::Signal, WeakRef};
    use gtk::CompositeTemplate;
    use once_cell::sync::Lazy;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Mousai/ui/song-page.ui")]
    pub struct SongPage {
        #[template_child]
        pub album_cover: TemplateChild<AlbumCover>,
        #[template_child]
        pub playback_button: TemplateChild<PlaybackButton>,
        #[template_child]
        pub last_heard_row: TemplateChild<InformationRow>,
        #[template_child]
        pub album_row: TemplateChild<InformationRow>,
        #[template_child]
        pub release_date_row: TemplateChild<InformationRow>,
        #[template_child]
        pub external_links_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub lyrics_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub lyrics_label: TemplateChild<gtk::Label>,

        pub song: RefCell<Option<Song>>,
        pub player: OnceCell<WeakRef<Player>>,
        pub bindings: RefCell<Vec<glib::Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SongPage {
        const NAME: &'static str = "MsaiSongPage";
        type Type = super::SongPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("song-page.toggle-playback", None, |obj, _, _| {
                if let Err(err) = obj.toggle_playback() {
                    log::warn!("Failed to toggle playback: {err:?}");
                    Application::default().show_error(&err.to_string());
                }
            });

            klass.install_action("song-page.remove-song", None, |obj, _, _| {
                if let Some(ref song) = obj.song() {
                    obj.emit_by_name::<()>("song-removed", &[song]);
                    obj.activate_action("win.navigate-to-main-page", None)
                        .unwrap();
                    obj.set_song(None);
                }
            });

            klass.install_action("song-page.copy-song", None, |obj, _, _| {
                if let Some(song) = obj.song() {
                    if let Some(display) = gdk::Display::default() {
                        display.clipboard().set_text(&format!(
                            "{} - {}",
                            song.artist(),
                            song.title()
                        ));

                        let toast = adw::Toast::new(&gettext("Copied song to clipboard"));
                        Application::default().add_toast(&toast);
                    }
                } else {
                    log::error!("Failed to copy song: There is no active song in SongPage");
                }
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SongPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder(
                    "song-removed",
                    &[Song::static_type().into()],
                    <()>::static_type().into(),
                )
                .build()]
            });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "song",
                    "Song",
                    "Song represented by Self",
                    Song::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "song" => {
                    let song = value.get().unwrap();
                    obj.set_song(song);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "song" => obj.song().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            self.external_links_box.connect_child_activated(|_, child| {
                let external_link_tile = child
                    .clone()
                    .downcast::<ExternalLinkTile>()
                    .expect("Expected `ExternalLinkTile` as child");

                utils::spawn(async move {
                    let external_link_wrapper = external_link_tile.external_link();
                    let external_link = external_link_wrapper.inner();
                    let uri = external_link.uri();

                    if let Err(err) = glib::Uri::is_valid(&uri, glib::UriFlags::ENCODED) {
                        log::warn!("Trying to launch an invalid Uri: {err:?}");
                    }

                    if let Err(err) = gio::AppInfo::launch_default_for_uri_future(
                        &uri,
                        gio::AppLaunchContext::NONE,
                    )
                    .await
                    {
                        log::warn!("Failed to launch default for uri `{uri}`: {err:?}");
                        Application::default()
                            .show_error(&gettext!("Failed to launch {}", external_link.name()));
                    }
                });
            });
        }

        fn dispose(&self, obj: &Self::Type) {
            while let Some(child) = obj.first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for SongPage {}
}

glib::wrapper! {
    pub struct SongPage(ObjectSubclass<imp::SongPage>)
        @extends gtk::Widget;
}

impl SongPage {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create SongPage")
    }

    pub fn connect_song_removed<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, &Song) + 'static,
    {
        self.connect_closure(
            "song-removed",
            true,
            closure_local!(|obj: &Self, song: &Song| {
                f(obj, song);
            }),
        )
    }

    pub fn set_song(&self, song: Option<Song>) {
        if song == self.song() {
            return;
        }

        let imp = self.imp();
        imp.song.replace(song.clone());

        {
            let mut bindings = imp.bindings.borrow_mut();

            for binding in bindings.drain(..) {
                binding.unbind();
            }

            if let Some(ref song) = song {
                bindings.push(
                    song.bind_property("lyrics", &imp.lyrics_label.get(), "label")
                        .flags(glib::BindingFlags::SYNC_CREATE)
                        .build(),
                );
                bindings.push(
                    song.bind_property("lyrics", &imp.lyrics_group.get(), "visible")
                        .transform_to(|_, value| {
                            let lyrics = value.get::<Option<String>>().unwrap();
                            Some(lyrics.is_some().to_value())
                        })
                        .flags(glib::BindingFlags::SYNC_CREATE)
                        .build(),
                );
            }
        }

        imp.album_cover.set_song(song);
        self.update_information();
        self.update_playback_ui();
        self.update_external_links();

        self.notify("song");
    }

    pub fn song(&self) -> Option<Song> {
        self.imp().song.borrow().clone()
    }

    /// Must only be called once.
    pub fn bind_player(&self, player: &Player) {
        player.connect_state_notify(clone!(@weak self as obj => move |_| {
            obj.update_playback_ui();
        }));

        self.imp().player.set(player.downgrade()).unwrap();

        self.update_playback_ui();
    }

    fn toggle_playback(&self) -> anyhow::Result<()> {
        if let Some(ref player) = self.imp().player.get().and_then(|player| player.upgrade()) {
            if let Some(song) = self.song() {
                if player.state() == PlayerState::Playing && player.is_active_song(&song) {
                    player.pause();
                } else {
                    player.set_song(Some(song))?;
                    player.play();
                }
            }
        }

        Ok(())
    }

    fn update_playback_ui(&self) {
        let imp = self.imp();
        let song = self.song();

        imp.playback_button.set_visible(
            song.as_ref()
                .and_then(|song| song.playback_link())
                .is_some(),
        );

        if let Some(ref song) = song {
            if let Some(player) = imp.player.get().and_then(|player| player.upgrade()) {
                let is_active_song = player.is_active_song(song);
                let player_state = player.state();

                if is_active_song && player_state == PlayerState::Buffering {
                    imp.playback_button.set_mode(PlaybackButtonMode::Buffering);
                } else if is_active_song && player_state == PlayerState::Playing {
                    imp.playback_button.set_mode(PlaybackButtonMode::Pause);
                } else {
                    imp.playback_button.set_mode(PlaybackButtonMode::Play);
                }
            } else {
                log::error!("Either the player was dropped or not binded in SongPage");
            }
        }
    }

    fn update_external_links(&self) {
        self.imp().external_links_box.bind_model(
            self.song().map(|song| song.external_links()).as_ref(),
            |item| {
                let wrapper: &ExternalLinkWrapper = item.downcast_ref().unwrap();
                ExternalLinkTile::new(wrapper).upcast()
            },
        );
    }

    fn update_information(&self) {
        let song = match self.song() {
            Some(song) => song,
            None => return,
        };

        let imp = self.imp();

        imp.last_heard_row
            .set_data(&song.last_heard().fuzzy_display());
        imp.album_row.set_data(&song.album());
        imp.release_date_row.set_data(&song.release_date());
    }
}

impl Default for SongPage {
    fn default() -> Self {
        Self::new()
    }
}
