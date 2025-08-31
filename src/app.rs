// SPDX-License-Identifier: GPL-3

use std::cmp::PartialEq;
use crate::apps::{get_installed_applications, get_startup_applications, DirectoryType};
use crate::fl;
use cosmic::app::{context_drawer, Core, Task};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Border, Color, Length, Subscription};
use cosmic::iced_core::widget::Text;
use cosmic::theme::Container::List;
use cosmic::widget::{self, button, column, container, icon, list_column, row, vertical_space};
use cosmic::{theme, Application, ApplicationExt, Apply, Element, Renderer, Theme};
use freedesktop_desktop_entry::DesktopEntry;
use futures_util::{FutureExt, SinkExt};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use cosmic::dialog::file_chooser::FileFilter;
use cosmic::widget::icon::IconFallback;

//const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,

    context_page: ContextPage,
    application_search: String,

    locales: Vec<String>,
    installed_apps: Option<Vec<DesktopEntry>>,

    apps_per_type: HashMap<DirectoryType, Vec<DesktopEntry>>,

    selected_type: Option<DirectoryType>,
    selected_app: Option<DesktopEntry>,

    // global search
    global_search: Option<String>,
    global_search_id: widget::Id,

    popover_item: Option<u32>,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    SubscriptionChannel,
    ToggleContextPage(ContextPage),

    ApplicationSearch(String),

    AddApplicationActivate(DirectoryType),
    AddApplication(DesktopEntry),

    RemoveApplication(DirectoryType, DesktopEntry),
    RemoveApplicationConfirm,
    RemoveApplicationCancel,

    // global search
    GlobalSearchActivate,
    GlobalSearchInput(String),
    GlobalSearchClear,

    ChooseScriptActivate(DirectoryType),
    ChooseScriptCancel,

    RefreshApps(DirectoryType),

    TogglePopover(u32),
    PopoverAction(u32, PopoverMessage)
}

#[derive(Clone, Debug)]
pub enum PopoverMessage {
    ViewInFiles,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    AddApplication,
}

/// Create a COSMIC application from the app model
impl Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "best.ellie.StartupConfiguration";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let locales = freedesktop_desktop_entry::get_languages_from_env();

        let mut apps_hash = HashMap::with_capacity(2);
        apps_hash.insert(
            DirectoryType::User,
            get_startup_applications(DirectoryType::User, locales.clone()),
        );
        apps_hash.insert(
            DirectoryType::System,
            get_startup_applications(DirectoryType::System, locales.clone()),
        );

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            locales: locales.clone(),
            installed_apps: None,
            application_search: String::new(),

            apps_per_type: apps_hash,

            selected_type: None,
            selected_app: None,

            global_search: None,
            global_search_id: widget::Id::unique(),

            popover_item: None,
        };

        // Create a startup command that sets the window title.
        let command = app.update_title();

        (app, command)
    }

    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::AddApplication => {
                let search = widget::search_input("Type to search...", &self.application_search)
                    .on_input(Message::ApplicationSearch)
                    .on_clear(Message::ApplicationSearch(String::new()));

                let search_input = &self.application_search.trim().to_lowercase();

                let mut list = list_column()
                    .padding(theme::active().cosmic().space_xs())
                    .list_item_padding(0);

                if let Some(installed_apps) = &self.installed_apps {
                    for application in installed_apps {
                        if search_input.is_empty()
                            || application
                            .name(&freedesktop_desktop_entry::get_languages_from_env())
                            .unwrap_or("".into())
                            .to_lowercase()
                            .contains(search_input)
                        {
                            let mut app_name = application.clone().appid;

                            if let Some(name) = &application.name(&self.locales) {
                                app_name = name.to_string();
                            }

                            let icon_name = application.icon().unwrap_or("application-default");
                            let mut icon = icon::from_name(icon_name);
                            icon.fallback = Some(IconFallback::Names(vec!["application-default".into()]));

                            let app_item_row = cosmic::iced::widget::row![
                            icon.size(24),
                            cosmic::iced::widget::column![
                                widget::text::heading(app_name),
                                exec_line(String::from(application.exec().unwrap_or("")))
                            ]
                            .width(Length::Fill),
                            widget::button::text(fl!("actions", "add"))
                                .on_press(Message::AddApplication(application.clone()))
                        ]
                                .spacing(theme::active().cosmic().space_xs())
                                .align_y(Vertical::Center);

                            list = list.add(app_item_row);
                        }
                    }
                }

                context_drawer::context_drawer(
                    cosmic::iced::widget::column![search, list]
                        .spacing(theme::active().cosmic().space_m()),
                    Message::ToggleContextPage(ContextPage::AddApplication),
                )
                .title(fl!("add-application"))
            }
        })
    }

    fn dialog(&self) -> Option<Element<Self::Message>> {
        if let Some(_selected_app) = &self.selected_app {
            return Some(
                widget::dialog()
                    .title(fl!("dialog-remove-application"))
                    .icon(icon::from_name("dialog-error-symbolic").size(64))
                    .body(fl!("dialog-remove-application", "body"))
                    .secondary_action(button::destructive(fl!("actions", "yes")).on_press(
                        Message::RemoveApplicationConfirm,
                    ))
                    .primary_action(
                        button::suggested(fl!("actions", "no"))
                            .on_press(Message::RemoveApplicationCancel),
                    )
                    .into(),
            );
        }
        None
    }

    fn header_end(&self) -> Vec<Element<Self::Message>> {
        let mut elements = Vec::with_capacity(2);

        if let Some(search) = &self.global_search {
            elements.push(
                widget::text_input::search_input("", search)
                    .width(Length::Fixed(240.0))
                    .id(self.global_search_id.clone())
                    .on_clear(Message::GlobalSearchClear)
                    .on_input(Message::GlobalSearchInput)
                    .into(),
            );
        } else {
            elements.push(
                widget::button::icon(icon::from_name("system-search-symbolic"))
                    .on_press(Message::GlobalSearchActivate)
                    .padding(8)
                    .selected(true)
                    .into(),
            );
        }

        elements
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They are started at the
    /// beginning of the application, and persist through its lifetime.
    fn subscription(&self) -> Subscription<Self::Message> {
        struct MySubscription;

        Subscription::batch(vec![
            // Create a subscription which emits updates through a channel.
            Subscription::run_with_id(
                std::any::TypeId::of::<MySubscription>(),
                cosmic::iced::stream::channel(4, move |mut channel| async move {
                    _ = channel.send(Message::SubscriptionChannel).await;

                    futures_util::future::pending().await
                }),
            ),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::SubscriptionChannel => {
                // For example purposes only.
            }
            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            }
            Message::ApplicationSearch(search) => {
                self.application_search = search;
            }
            Message::AddApplicationActivate(directory_type) => {
                if self.installed_apps.is_none() {
                    self.installed_apps = Some(get_installed_applications(self.locales.clone()));
                }
                self.selected_type = Some(directory_type);
                return cosmic::task::message(Message::ToggleContextPage(ContextPage::AddApplication));
            }
            Message::AddApplication(desktop_entry) => {
                if let Some(directory_type) = &self.selected_type {
                    let mut file_name = desktop_entry.clone().appid;
                    file_name.push_str(".desktop");

                    let directories: Vec<PathBuf> = directory_type.clone().into();

                    let directory_to_target = directories.get(0).expect("Always at least one directory");

                    if let Ok(exists) = std::fs::exists(directory_to_target.join(file_name.clone())) {
                        if !exists {
                            #[cfg(feature = "flatpak")]
                            match fs::copy(
                                desktop_entry.clone().path,
                                directory_to_target.join(file_name),
                            ) {
                                Ok(_) => {
                                    self.apps_per_type.insert(directory_type.clone(), get_startup_applications(directory_type.clone(), self.locales.clone()));
                                }
                                Err(e) => {
                                    // @todo - error handling
                                }
                            }

                            #[cfg(not(feature = "flatpak"))]
                            match std::os::unix::fs::symlink(
                                desktop_entry.clone().path,
                                directory_to_target.join(file_name),
                            ) {
                                Ok(_) => {
                                    self.apps_per_type.insert(directory_type.clone(), get_startup_applications(directory_type.clone(), self.locales.clone()));
                                }
                                Err(e) => {
                                    // @todo - error handling
                                }
                            }
                        }
                    }
                }

                self.selected_type = None;
                return cosmic::task::message(Message::ToggleContextPage(ContextPage::AddApplication));
            }
            Message::RemoveApplication(directory_type, desktop_entry) => {
                self.selected_type = Some(directory_type);
                self.selected_app = Some(desktop_entry);
            }
            Message::RemoveApplicationConfirm => {
                if let Some(directory_type) = &self.selected_type {
                    if let Some(desktop_entry) = &self.selected_app {
                        let mut file_name = desktop_entry.clone().appid;
                        file_name.push_str(".desktop");

                        let directories: Vec<PathBuf> = directory_type.clone().into();

                        let directory_to_target = directories.get(0).expect("Always at least one directory");

                        if let Ok(exists) = std::fs::exists(directory_to_target.join(file_name.clone())) {
                            if exists {
                                match std::fs::remove_file(
                                    directory_to_target.join(file_name),
                                ) {
                                    Ok(_) => {
                                        self.apps_per_type.insert(directory_type.clone(), get_startup_applications(directory_type.clone(), self.locales.clone()));
                                    }
                                    Err(e) => {
                                        // @todo - error handling
                                    }
                                }
                            }
                        }
                    }
                }

                self.selected_type = None;
                self.selected_app = None;

            }
            Message::RemoveApplicationCancel => {
                self.selected_type = None;
                self.selected_app = None;
            }
            Message::GlobalSearchActivate => {
                self.global_search = Some(String::new());
                return widget::text_input::focus(self.global_search_id.clone());
            }
            Message::GlobalSearchInput(search) => {
                self.global_search = Some(search);
            }
            Message::GlobalSearchClear => {
                self.global_search = None;
            }
            Message::ChooseScriptActivate(directory_type) => {
                self.selected_type = Some(directory_type.clone());
                return cosmic::dialog::file_chooser::open::Dialog::new()
                    .directory(dirs::home_dir().unwrap())
                    .title(fl!("script-chooser"))
                    .filter(
                        FileFilter::new("*sh scripts")
                            .glob("*.*sh")
                    )
                    .filter(
                        FileFilter::new("Python scripts")
                            .glob("*.py*")
                    )
                    .filter(
                        FileFilter::new("All files")
                            .glob("*.*")
                    )
                    .open_file()
                    .then(|result| async move {
                        return match result {
                            Ok(response) => {
                                println!("{:?}", response.0);
                                let Ok(path) = response.url().to_file_path() else {
                                    // @todo - error
                                    return Message::ChooseScriptCancel;
                                };

                                // spaghetti?
                                let Some(file_name) = path.file_name() else {
                                    // @todo - error
                                    return Message::ChooseScriptCancel;
                                };
                                let Some(file_name) = file_name.to_str() else {
                                    // @todo - error
                                    return Message::ChooseScriptCancel;
                                };

                                let directories: Vec<PathBuf> = directory_type.clone().into();

                                let directory_to_target = directories.get(0).expect("Always at least one directory");

                                // when bundled as a flatpak, we need to do a few creative things..
                                // 1) write the script itself to `~/.config/autostart/` so it's in a
                                // known location - flatpaks do not let us get the host path!
                                // 2) set the script as +x
                                #[cfg(feature = "flatpak")]
                                match std::fs::write(directory_to_target.join(file_name), std::fs::read_to_string(path.clone()).expect("Should be able to read")) {
                                    Ok(_) => {
                                        let entry_text = format!("[Desktop Entry]
Type=Application
Name={}
Exec=sh -c '~/.config/autostart/{}'", file_name, file_name);
                                        let mut desktop_file_name = file_name.to_string();
                                        desktop_file_name.push_str(".desktop");
                                        match std::fs::write(directory_to_target.join(desktop_file_name), entry_text) {
                                            Ok(_) => {
                                                if let Ok(metadata) = fs::metadata(directory_to_target.join(file_name)) {
                                                    use std::os::unix::fs::PermissionsExt;

                                                    let mut permissions = metadata.permissions();
                                                    permissions.set_mode(permissions.mode() | 0o111);
                                                    if let Ok(_) = fs::set_permissions(
                                                        directory_to_target.join(file_name),
                                                        permissions
                                                    ) {
                                                        return Message::RefreshApps(directory_type);
                                                    }
                                                }

                                            }
                                            Err(err) => {
                                                // @ todo - error
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        // @todo - error
                                    }
                                }

                                #[cfg(not(feature = "flatpak"))]
                                {
                                    let entry_text = format!("[Desktop Entry]
Type=Application
Name={}
Exec={:?}", file_name, path);
                                    let mut desktop_file_name = file_name.to_string();
                                    desktop_file_name.push_str(".desktop");
                                    match std::fs::write(directory_to_target.join(desktop_file_name), entry_text) {
                                        Ok(_) => {
                                            return Message::RefreshApps(directory_type);
                                        }
                                        Err(err) => {
                                            // @ todo - error
                                        }
                                    }
                                }

                                Message::ChooseScriptCancel
                            }
                            Err(cosmic::dialog::file_chooser::Error::Cancelled) => {
                                Message::ChooseScriptCancel
                            }
                            Err(why) => {
                                Message::ChooseScriptCancel
                            }
                        }
                    })
                    .apply(cosmic::task::future);
            }
            Message::ChooseScriptCancel => {}
            Message::RefreshApps(directory_type) => {
                self.apps_per_type.insert(directory_type.clone(), get_startup_applications(directory_type.clone(), self.locales.clone()));
            }
            Message::TogglePopover(idx) => {
                if let Some(current_idx) = self.popover_item {
                    if idx == current_idx {
                        self.popover_item = None;
                    }
                    else {
                        self.popover_item = Some(idx);
                    }
                }
                else {
                    self.popover_item = Some(idx);
                }
            }
            Message::PopoverAction(idx, popover_action) => {
                if let Some(user_apps) = self.apps_per_type.get(&DirectoryType::User) {
                    if let Some(app) = user_apps.get(idx as usize) {
                        match popover_action {
                            PopoverMessage::ViewInFiles => {
                                if let Some(dir) = &app.path.parent() {
                                    // when run as a flatpak, we need to de-sandbox the directory to try and guess
                                    // the hosts' autostart directory
                                    #[cfg(feature = "flatpak")]
                                    if dir.starts_with("/home") && dir.components().any(|c| c.as_os_str() == ".var") {
                                        if let Some(dir) = dir.as_os_str().to_str() {
                                            if let Some(home_dir) = dir.split(".var").nth(0) {
                                                showfile::show_path_in_file_manager(
                                                    PathBuf::from(home_dir)
                                                        .join(".config/autostart/")
                                                        .join(app.path.file_name().expect("We should always have this")),
                                                );
                                            }
                                        }
                                    }
                                    else {
                                        showfile::show_path_in_file_manager(&app.path);
                                    }

                                    #[cfg(not(feature = "flatpak"))]
                                    showfile::show_path_in_file_manager(&app.path);
                                }
                                
                            }
                        }
                    }
                }
                
                self.popover_item = None;
            }
        }
        Task::none()
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<Self::Message> {
        let cosmic::cosmic_theme::Spacing {
            space_s,
            space_xs,
            space_l,
            ..
        } = theme::active().cosmic().spacing;

        let mut sections = column().spacing(space_l);

        let header = column()
            .push(widget::text::title1(fl!("window-title")))
            .push(widget::text(fl!("application-description")));

        sections = sections.push(header);

        let available_types = vec![DirectoryType::User, DirectoryType::System];

        for directory_type in available_types {
            let mut section = column().spacing(space_s);

            let (section_name, section_description) = match directory_type {
                DirectoryType::User => (
                    fl!("user-applications"),
                    fl!("user-applications", "description"),
                ),
                DirectoryType::System => (
                    fl!("system-applications"),
                    fl!("system-applications", "description"),
                ),
            };

            section = section.push(
                column()
                    .push(widget::text::heading(section_name).size(18.0))
                    .push(widget::text(section_description)),
            );

            let mut valid_apps = 0;
            let search_input = match &self.global_search {
                None => "",
                Some(search) => &search.trim().to_lowercase(),
            };

            if let Some(apps) = self.apps_per_type.get(&directory_type) {
                let is_user = directory_type == DirectoryType::User;
                if apps.len() > 0 {
                    let mut list_col = list_column().style(List);
                    let mut idx = 0;
                    for app in apps {
                        let app_name = match app.name(&self.locales) {
                            Some(name) => name.to_string(),
                            None => app.appid.to_owned(),
                        };

                        let app_exec = app.exec().expect("invalid state");

                        if search_input.is_empty()
                            || app_name.to_lowercase().contains(search_input)
                            || app_exec.to_lowercase().contains(search_input)
                        {
                            valid_apps = valid_apps + 1;

                            let mut row = row::with_capacity(3)
                                .spacing(space_xs)
                                .align_y(Alignment::Center);

                            let mut icon = icon::from_name(app.icon().unwrap_or("application-default"));
                            icon.fallback = Some(IconFallback::Names(vec!["application-default".into()]));
                            
                            row = row.push(icon.size(32));

                            let mut name_col = column().align_x(Alignment::Start);

                            name_col =
                                name_col.push(widget::text::heading(app_name).width(Length::Fill));
                            name_col = name_col.push(exec_line(String::from(app_exec)));

                            row = row.push(name_col);

                            // actions
                            if is_user {
                                let is_expanded = match self.popover_item {
                                    Some(i) => i == idx,
                                    None => false
                                };
                                let more_button = button::icon(icon::from_name("view-more-symbolic"))
                                    .on_press(Message::TogglePopover(idx))
                                    .extra_small();

                                let mut actions_row = widget::row()
                                    .spacing(space_xs)
                                    .push(
                                        button::icon(icon::from_name("edit-delete-symbolic"))
                                            .extra_small()
                                            .on_press(Message::RemoveApplication(directory_type.clone(), app.clone())),
                                    );

                                if is_expanded {
                                    actions_row = actions_row.push(cosmic::widget::popover(more_button)
                                        .popup(column::with_children(vec![
                                            popover_item(idx, fl!("popover-menu", "view-in-files"), PopoverMessage::ViewInFiles),
                                        ])
                                            .padding([2, 8])
                                            .width(Length::Shrink)
                                            .height(Length::Shrink)
                                            .apply(widget::container)
                                            .class(theme::Container::custom(|theme| {
                                                let cosmic = theme.cosmic();
                                                let background = &cosmic.background;

                                                container::Style {
                                                    icon_color: Some(background.on.into()),
                                                    text_color: Some(background.on.into()),
                                                    background: Some(Color::from(background.base).into()),
                                                    border: Border {
                                                        color: background.component.divider.into(),
                                                        width: 1.0,
                                                        radius: cosmic.corner_radii.radius_s.into(),
                                                        ..Border::default()
                                                    },
                                                    shadow: Default::default(),
                                                }
                                            }))
                                        )
                                        .on_close(Message::TogglePopover(idx)));
                                }
                                else {
                                    actions_row = actions_row.push(more_button);
                                }

                                row = row.push(
                                    actions_row
                                );
                            }

                            list_col = list_col.add(row);
                        }

                        idx += 1;
                    }

                    if valid_apps > 0 {
                        section = section.push(list_col);
                    } else {
                        section = section.push(
                            list_column()
                                .style(List)
                                .add(widget::text::heading(fl!("no-applications-found"))),
                        );
                    }
                } else {
                    section = section.push(
                        list_column().style(List).add(
                            cosmic::iced::widget::column![
                                widget::text::title3(fl!("no-applications-selected")),
                                widget::text::caption(fl!("no-applications-caption"))
                            ]
                            .width(Length::Fill)
                            .align_x(Horizontal::Center),
                        ),
                    );
                }

                // @todo: get directory type
                if search_input.is_empty() && is_user {
                    let controls = widget::container(
                        row()
                            .spacing(space_xs)
                            .push(
                                button::standard(fl!("add-script")).trailing_icon(
                                    icon::from_name("window-pop-out-symbolic"),
                                )
                                    .on_press(Message::ChooseScriptActivate(directory_type.clone())),
                            )
                            .push(
                                button::suggested(fl!("add-application"))
                                    .trailing_icon(icon::from_name("list-add-symbolic"))
                                    .on_press(Message::AddApplicationActivate(directory_type.clone())),
                            ),
                    )
                        .width(Length::Fill)
                        .align_x(Alignment::End);
                    section = section.push(controls);
                }
            }

            sections = sections.push(section);
        }

        sections = sections.push(vertical_space().height(Length::Fixed(64.0)));

        widget::container(
            widget::scrollable(sections)
                .height(Length::Fill)
                .spacing(space_l),
        )
        // fill the full application window
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Left)
        .align_y(Vertical::Top)
        .padding([0, 0, 0, space_l])
        .into()
    }
}

impl AppModel {
    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Task<Message> {
        let window_title = fl!("app-title");

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }
}

fn exec_line<'a>(text: String) -> Text<'a, Theme, Renderer> {
    widget::text::monotext(text).size(10.0)
}

fn popover_item(idx: u32, label: String, message: PopoverMessage) -> Element<'static, Message> {
    widget::text::body(label)
        .apply(widget::container)
        .class(theme::Container::custom(|theme| {
            container::Style {
                background: None,
                ..container::Catalog::style(theme, &List)
            }
        }))
        .apply(button::custom)
        .on_press(Message::PopoverAction(idx, message))
        .class(theme::Button::Transparent)
        .into()
}