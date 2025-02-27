use gtk::{
    gdk,
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};

use std::cell::RefCell;

use crate::{model::Song, utils};

mod imp {
    use super::*;
    use gtk::CompositeTemplate;
    use once_cell::sync::Lazy;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/seadve/Mousai/ui/album-cover.ui")]
    pub struct AlbumCover {
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub image_a: TemplateChild<gtk::Image>,
        #[template_child]
        pub image_b: TemplateChild<gtk::Image>,
        #[template_child]
        pub placeholder: TemplateChild<gtk::Image>,

        pub song: RefCell<Option<Song>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumCover {
        const NAME: &'static str = "MsaiAlbumCover";
        type Type = super::AlbumCover;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AlbumCover {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "song",
                        "Song",
                        "Song represented by Self",
                        Song::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpecInt::new(
                        "pixel-size",
                        "Pixel Size",
                        "Pixel Size of the inner GtkImage",
                        -1,
                        i32::MAX,
                        -1,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                ]
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
                "pixel-size" => {
                    let pixel_size = value.get().unwrap();
                    obj.set_pixel_size(pixel_size);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "song" => obj.song().to_value(),
                "pixel-size" => obj.pixel_size().to_value(),
                _ => unimplemented!(),
            }
        }

        fn dispose(&self, obj: &Self::Type) {
            while let Some(child) = obj.first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for AlbumCover {}
}

glib::wrapper! {
    pub struct AlbumCover(ObjectSubclass<imp::AlbumCover>)
        @extends gtk::Widget;
}

impl AlbumCover {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create AlbumCover")
    }

    pub fn set_song(&self, song: Option<Song>) {
        if song == self.song() {
            return;
        }

        if let Some(ref song) = song {
            match song.album_art() {
                Ok(album_art) => {
                    utils::spawn(clone!(@weak self as obj, @weak album_art => async move {
                        match album_art.texture().await {
                            Ok(texture) => {
                                obj.set_paintable(texture);
                            }
                            Err(err) => {
                                log::warn!("Failed to load texture: {err:?}");
                                obj.clear();
                            }
                        }
                    }));
                }
                Err(err) => {
                    log::warn!("Failed to get song album art: {err:?}");
                    self.clear();
                }
            }
        } else {
            self.clear();
        }

        self.imp().song.replace(song);
        self.notify("song");
    }

    pub fn song(&self) -> Option<Song> {
        self.imp().song.borrow().clone()
    }

    pub fn set_pixel_size(&self, pixel_size: i32) {
        let imp = self.imp();
        imp.image_a.set_pixel_size(pixel_size);
        imp.image_b.set_pixel_size(pixel_size);
        imp.placeholder.set_pixel_size(pixel_size / 3);
        self.notify("pixel-size");
    }

    pub fn pixel_size(&self) -> i32 {
        self.imp().image_a.pixel_size()
    }

    fn clear(&self) {
        let imp = self.imp();
        imp.image_a.set_paintable(gdk::Paintable::NONE);
        imp.image_b.set_paintable(gdk::Paintable::NONE);
        imp.stack.set_visible_child(&imp.placeholder.get());
    }

    fn set_paintable(&self, paintable: &impl IsA<gdk::Paintable>) {
        let imp = self.imp();

        if imp.stack.visible_child().as_ref() == Some(imp.image_a.upcast_ref()) {
            imp.image_b.set_paintable(Some(paintable));
            imp.stack.set_visible_child(&imp.image_b.get());
        } else {
            imp.image_a.set_paintable(Some(paintable));
            imp.stack.set_visible_child(&imp.image_a.get());
        }
    }
}

impl Default for AlbumCover {
    fn default() -> Self {
        Self::new()
    }
}
