// SPDX-License-Identifier: GPL-3

use std::fs;
use std::path::PathBuf;
use crate::app::Message::ToggleContextPage;
use crate::config::Config;
use crate::fl;
use crate::programs::{get_installed_programs, get_program_list, Program, ProgramSettings};
use cosmic::app::{context_drawer, Core, Task};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Length, Subscription};
use cosmic::widget::{self, icon};
use cosmic::{theme, Application, ApplicationExt, Element};
use futures_util::SinkExt;

//const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
//const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,
    // Configuration data that persists between application runs.
    config: Config,

    context_page: ContextPage,
    application_search: String,

    installed_programs: Vec<Program>,
    selected_programs: Option<Vec<Program>>,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    SubscriptionChannel,
    UpdateConfig(Config),
    ToggleContextPage(ContextPage),

    ApplicationSearch(String),
    AddApplication(Program),
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    AddApplication,
}

fn application_item(icon_size: u16, title_size: u16, program: &Program) -> Element<Message> {
    cosmic::iced::widget::row![
        icon::from_name(&*program.icon).size(icon_size),
        cosmic::iced::widget::column![
            widget::text::title3(&program.name).size(title_size),
            widget::text::caption(&program.settings.exec)
        ]
    ].spacing(theme::active().cosmic().space_m()).into()
}

/// Create a COSMIC application from the app model
impl Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = Vec<ProgramSettings>;

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "best.ellie.CosmicStartup";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            installed_programs: get_installed_programs(),
            application_search: String::new(),
            // Optional configuration file for an application.
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| Config::get_entry(&context).unwrap_or_else(|(_errors, config)| {
                    // for why in errors {
                    //     tracing::error!(%why, "error loading app config");
                    // }

                    config
                }))
                .unwrap_or_default(),
            selected_programs: match _flags.len() {
                0 => None,
                _ => Some(get_program_list(&_flags))
            }
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
                let search = widget::search_input("Type to search...", &self.application_search).on_input(Message::ApplicationSearch).on_clear(Message::ApplicationSearch(String::new()));

                let search_input = &self.application_search.trim().to_lowercase();

                let mut list = widget::list_column();

                for program in self.installed_programs.iter() {
                    if search_input.is_empty() || (program.name).to_lowercase().contains(search_input) {
                        list = list.add(widget::button::custom(
                            application_item(theme::active().cosmic().space_m(), theme::active().cosmic().space_l(), program)
                        ).on_press(Message::AddApplication(program.clone())));
                    }
                }

                context_drawer::context_drawer(
                    cosmic::iced::widget::column![
                        search,
                        list
                    ],
                    Message::ToggleContextPage(ContextPage::AddApplication)
                ).title(fl!("add-application"))
            }
        })
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<Self::Message> {
        let mut list_col = widget::list_column().style(theme::Container::List);

        if self.selected_programs.is_some() {
            list_col = self.selected_programs.as_deref().unwrap().iter().fold(list_col, |list, program| {
                let mut s = String::new();
                s.push_str(&*program.name);
                //s.push_str(&*program.exec);

                let exec = String::from(&*program.settings.exec);

                list.add(
                    cosmic::iced::widget::row![
                    cosmic::widget::icon::from_name(&*program.icon).size(theme::active().cosmic().space_l()),
                    widget::Space::with_width(theme::active().cosmic().space_m()),
                    cosmic::iced::widget::column![
                        widget::text::title3(s),
                        widget::text::caption(exec)
                    ]
                ]
                )
            });
        }
        else {
            list_col = list_col.add(
                cosmic::iced::widget::column![
                    widget::text::title3(fl!("no-applications-selected")),
                    widget::text::caption(fl!("no-applications-caption"))
                ].width(Length::Fill).align_x(Horizontal::Center)
            );
        }


        widget::container(
            cosmic::iced::widget::column![
                    widget::text::title1(fl!("window-title")),
                    widget::text::text(fl!("application-description")),
                    widget::Space::with_height(theme::active().cosmic().space_l()),
                    cosmic::iced::widget::column![
                        widget::button::suggested(fl!("add-application")).trailing_icon(icon::from_name("list-add-symbolic")).on_press(Message::ToggleContextPage(ContextPage::AddApplication)),
                    ].width(Length::Fill).align_x(Horizontal::Right).padding(theme::active().cosmic().space_s()),
                    widget::scrollable(list_col).height(Length::Fill)
            ]
        )
            // fill the full application window
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Left)
            .align_y(Vertical::Top)
            .padding(cosmic::theme::active().cosmic().space_l())
            .into()
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
            // Watch for application configuration changes.
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| {
                    // for why in update.errors {
                    //     tracing::error!(?why, "app config error");
                    // }

                    Message::UpdateConfig(update.config)
                }),
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
            Message::UpdateConfig(config) => {
                self.config = config;
            },
            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    self.core.window.show_context = !self.core.window.show_context;
                }
                else {
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            },
            Message::ApplicationSearch(search) => {
                self.application_search = search;
            },
            Message::AddApplication(program) => {
                let mut new_programs = self.selected_programs.take().unwrap_or(Vec::new());
                new_programs.push(program);

                self.selected_programs = Some(new_programs);

                let mut settings = Vec::new();
                for program in self.selected_programs.as_deref().unwrap() {
                    settings.push(&program.settings);
                }

                //todo: refactor
                #[allow(deprecated)]
                let mut home_dir = std::env::home_dir().unwrap_or(PathBuf::from("/home/"));
                home_dir.push(".config");
                home_dir.push("cosmic-startup.ron");
                fs::write(&home_dir, ron::to_string(&settings).unwrap()).unwrap();

                if self.context_page == ContextPage::AddApplication {
                    return cosmic::task::message(ToggleContextPage(ContextPage::AddApplication));
                }
            }
        }
        Task::none()
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