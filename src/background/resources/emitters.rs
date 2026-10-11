use std::sync::Arc;

use seelen_core::{
    handlers::SeelenEvent,
    resource::ResourceKind,
    state::{Theme, Widget, settings::shortcuts::resolve_shortcuts},
};
use slu_ipc::messages::SvcAction;

use crate::{
    app::emit_to_webviews,
    cli::ServicePipe,
    error::{Result, ResultLogExt},
    state::application::FULL_STATE,
    widgets::manager::WIDGET_MANAGER,
    widgets::permissions::WIDGET_PERMISSIONS,
    widgets::popups::shortcut_conflicts::show_shortcut_conflict_popup,
};

use super::ResourceManager;

// Reusable getters for resources.
impl ResourceManager {
    pub fn widgets(&self) -> Vec<Arc<Widget>> {
        let mut widgets = Vec::new();
        self.widgets.iter_sync(|_, v| {
            if v.metadata.premium {
                return true;
            }
            widgets.push(v.clone());
            true
        });
        widgets
    }

    pub fn themes(&self) -> Vec<Arc<Theme>> {
        let mut themes = Vec::new();
        self.themes.iter_sync(|_, v| {
            if v.metadata.premium {
                return true;
            }
            themes.push(v.clone());
            true
        });
        themes
    }

    pub fn plugins(&self) -> Vec<Arc<seelen_core::state::Plugin>> {
        let mut plugins = Vec::new();
        self.plugins.iter_sync(|_, v| {
            if v.metadata.premium {
                return true;
            }
            plugins.push(v.clone());
            true
        });
        plugins
    }

    pub fn icon_packs(&self) -> Vec<Arc<seelen_core::state::IconPack>> {
        let mut icon_packs = Vec::new();
        // Add system icon pack if it exists
        if let Some(system_pack) = self.system_icon_pack.lock().as_ref() {
            icon_packs.push(std::sync::Arc::new(system_pack.clone()));
        }
        // Add user icon packs
        self.icon_packs.iter_sync(|_, v| {
            if v.metadata.premium {
                return true;
            }
            icon_packs.push(v.clone());
            true
        });
        icon_packs
    }

    pub fn wallpapers(&self) -> Vec<Arc<seelen_core::state::Wallpaper>> {
        let mut wallpapers = Vec::new();
        self.wallpapers.iter_sync(|_, v| {
            if v.metadata.premium {
                return true;
            }
            wallpapers.push(v.clone());
            true
        });
        wallpapers
    }
}

impl ResourceManager {
    pub fn emit_widgets(&self) -> Result<()> {
        let widgets = self.widgets();
        emit_to_webviews(SeelenEvent::StateWidgetsChanged, widgets.clone());
        WIDGET_PERMISSIONS.remove_uninstalled_widgets().log_error();

        WIDGET_MANAGER.reconcile()?;

        let state = FULL_STATE.load();
        let widget_refs: Vec<_> = widgets.iter().map(|w| w.as_ref()).collect();
        let (resolved, has_conflicts) = resolve_shortcuts(&state.settings, &widget_refs);
        if has_conflicts {
            show_shortcut_conflict_popup().log_error();
        }
        if !crate::cli::shortcuts::SHORTCUTS_PAUSED.load(std::sync::atomic::Ordering::Acquire) {
            ServicePipe::request(SvcAction::SetShortcuts(resolved))?;
        }
        Ok(())
    }

    pub fn emit_themes(&self) {
        emit_to_webviews(SeelenEvent::StateThemesChanged, self.themes())
    }

    pub fn emit_plugins(&self) {
        emit_to_webviews(SeelenEvent::StatePluginsChanged, self.plugins())
    }

    pub fn emit_icon_packs(&self) {
        emit_to_webviews(SeelenEvent::StateIconPacksChanged, self.icon_packs())
    }

    pub fn emit_wallpapers(&self) {
        emit_to_webviews(SeelenEvent::StateWallpapersChanged, self.wallpapers())
    }

    pub fn emit_kind_changed(&self, kind: &ResourceKind) -> Result<()> {
        match kind {
            ResourceKind::Theme => self.emit_themes(),
            ResourceKind::Widget => {
                self.emit_plugins();
                self.emit_widgets()?;
            }
            ResourceKind::Plugin => self.emit_plugins(),
            ResourceKind::IconPack => self.emit_icon_packs(),
            ResourceKind::Wallpaper => self.emit_wallpapers(),
            ResourceKind::SoundPack => {
                // feature not implemented
            }
        }
        Ok(())
    }
}
