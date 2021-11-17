use enso_frp as frp;
use ensogl::application::Application;
use ensogl::application::{self};
use ensogl::display::shape::StyleWatchFrp;
use ensogl::display::DomSymbol;
use ensogl::display::{self};
use ensogl::prelude::*;
use ensogl::system::web::AttributeSetter;
use ensogl::system::web::NodeInserter;
use ensogl::system::web::StyleSetter;
use ensogl::system::web::{self};
use std::rc::Rc;
use utils::fail::FallibleResult;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;
use web_sys::MouseEvent;

type ClickClosure = Closure<dyn FnMut(MouseEvent)>;

#[derive(Clone, CloneRef, Debug)]
#[allow(missing_docs)]
pub struct Model {
    application:             Application,
    logger:                  Logger,
    dom:                     DomSymbol,
    display_object:          display::object::Instance,
    closures:                Rc<RefCell<Vec<ClickClosure>>>,
    projects_list:           Rc<web_sys::Element>,
    project_list_new_button: Rc<web_sys::Element>,
}

impl Model {
    pub fn new(app: &Application, frp: Frp) -> Self {
        let application = app.clone_ref();
        let logger = Logger::new("WelcomeScreen");
        let display_object = display::object::Instance::new(&logger);

        let mut closures = Vec::new();
        let (welcome_screen, projects_list, new_project) = {
            let welcome_screen = web::create_div();
            welcome_screen.set_class_name("templates-view");
            welcome_screen.set_id("templates-view");

            let (container, projects_list, new_project) = {
                let container = web::create_div();
                container.set_class_name("container");

                let (side_menu, projects_list, new_project) = Self::create_side_menu(&mut closures, frp);
                container.append_or_panic(&side_menu);
                container
                    .append_or_panic(&Self::create_templates(&mut closures, logger.clone_ref()));

                (container, projects_list, new_project)
            };
            welcome_screen.append_or_panic(&container);
            (welcome_screen, projects_list, new_project)
        };

        let dom = DomSymbol::new(&welcome_screen);
        display_object.add_child(&dom);
        app.display.scene().dom.layers.back.manage(&dom);
        // Use `panel` layer to lock position when panning
        app.display.scene().layers.panel.add_exclusive(&dom);


        let model = Self {
            application,
            logger,
            dom,
            display_object,
            projects_list: Rc::new(projects_list),
            project_list_new_button: Rc::new(new_project),
            closures: Rc::new(RefCell::new(closures)),
        };

        model
    }

    fn create_side_menu(closures: &mut Vec<ClickClosure>, frp: Frp) -> (web_sys::Element, web_sys::Element, web_sys::Element) {
        let side_menu = web::create_element("aside");
        side_menu.set_class_name("side-menu");
        let header = {
            let header = web::create_element("h2");
            header.set_text_content(Some("Your projects"));
            header
        };
        side_menu.append_or_panic(&header);

        let new_project;
        let projects_list = {
            let projects_list = web::create_element("ul");
            projects_list.set_id("projects-list");

            new_project = web::create_element("li");
            new_project.set_id("projects-list-new-project");
            new_project
                .set_inner_html(r#"<img src="/assets/new-project.svg" />Create a new project"#);

            let closure = Box::new(move |_event: MouseEvent| {
                frp.create_project.emit(());
            });
            let closure: Closure<dyn FnMut(MouseEvent)> = Closure::wrap(closure);
            let callback = closure.as_ref().unchecked_ref();
            new_project
                .add_event_listener_with_callback("click", callback)
                .expect("Unable to add event listener");
            closures.push(closure);

            projects_list.append_or_panic(&new_project);

            projects_list
        };
        side_menu.append_or_panic(&projects_list);

        (side_menu, projects_list, new_project)
    }

    fn create_templates(closures: &mut Vec<ClickClosure>, logger: Logger) -> web_sys::Element {
        let content = web::create_element("main");
        content.set_class_name("content");

        let templates = {
            let templates = web::create_div();
            let header = {
                let header = web::create_element("h2");
                header.set_text_content(Some("Templates"));
                header
            };
            templates.append_or_panic(&header);
            templates.append_or_panic(&Self::create_cards(closures, logger));
            templates
        };
        content.append_or_panic(&templates);

        content
    }

    fn create_cards(closures: &mut Vec<ClickClosure>, logger: Logger) -> web_sys::HtmlDivElement {
        let cards = web::create_div();
        cards.set_class_name("cards");

        let row1 = {
            let row = web::create_div();
            row.set_class_name("row");
            let card_spreadsheets = Self::create_card(
                "card-spreadsheets",
                "card card-spreadsheets",
                Some("/assets/spreadsheets.png"),
                "Combine spreadsheets",
                "Glue multiple spreadsheets together to analyse all your data at once.",
            );

            let closure = Box::new(move |_event: MouseEvent| {
                info!(logger, "Closure!");
                web_sys::console::log_1(&"Closure web_sys!".into());
            });
            let closure: Closure<dyn FnMut(MouseEvent)> = Closure::wrap(closure);
            let callback = closure.as_ref().unchecked_ref();
            card_spreadsheets
                .add_event_listener_with_callback("click", callback)
                .expect("Unable to add event listener");
            closures.push(closure);

            let card_geo = Self::create_card(
                "card-geo",
                "card card-geo",
                None,
                "Geospatial analysis",
                "Learn where to open a coffee shop to maximize your income.",
            );
            row.append_or_panic(&card_spreadsheets);
            row.append_or_panic(&card_geo);

            row
        };
        let row2 = {
            let row = web::create_div();
            row.set_class_name("row");
            let card_visualize = Self::create_card(
                "card-visualize",
                "card card-visualize",
                None,
                "Analyze GitHub stars",
                "Find out which of Enso's repositories are most popular over time.",
            );
            row.append_or_panic(&card_visualize);

            row
        };
        cards.append_or_panic(&row1);
        cards.append_or_panic(&row2);

        cards
    }

    fn create_card(
        id: &str,
        class: &str,
        img: Option<&str>,
        header: &str,
        content: &str,
    ) -> web_sys::HtmlDivElement {
        let card = web::create_div();
        card.set_id(id);
        card.set_class_name(class);
        if let Some(src) = img {
            let img = web::create_element("img");
            img.set_attribute_or_panic("src", src);
            card.append_or_panic(&img);
        }
        let card_header = web::create_element("h3");
        card_header.set_text_content(Some(header));
        card.append_or_panic(&card_header);
        let p = web::create_element("p");
        p.set_text_content(Some(content));
        card.append_or_panic(&p);

        card
    }

    fn update_projects_list(&self, projects: &[String], frp: Frp) -> FallibleResult {
        let projects_list = self.projects_list.clone_ref();
        let new_project_button = self.project_list_new_button.clone_ref();
        for project in projects {
            let project = project.clone();
            let node = web::create_element("li");
            node.set_inner_html(&iformat!(r#"<img src="assets/project.svg"/> {project}"#));
            node.set_attribute_or_warn("style", "cursor: pointer", &self.logger);
            let frp = frp.clone_ref();
            let closure = Box::new(move |_event: MouseEvent| {
                frp.open_project.emit(project.clone());
            });
            let closure: Closure<dyn FnMut(MouseEvent)> = Closure::wrap(closure);
            let callback = closure.as_ref().unchecked_ref();
            node.add_event_listener_with_callback("click", callback)
                .expect("Unable to add event listener");
            self.closures.borrow_mut().push(closure);
            projects_list.insert_before_or_warn(&node, &new_project_button, &self.logger);
        }
        Ok(())
    }
}

ensogl::define_endpoints! {
    Input {
        projects_list(Vec<String>),

        open_project(String),
        create_project(),
        init(),
    }

    Output {
        opened_project(Option<String>),
    }
}


#[derive(Clone, CloneRef, Debug)]
pub struct View {
    model:   Model,
    styles:  StyleWatchFrp,
    pub frp: Frp,
}

impl Deref for View {
    type Target = Frp;
    fn deref(&self) -> &Self::Target {
        &self.frp
    }
}

impl View {
    pub fn new(app: &Application) -> Self {
        let frp = Frp::new();
        let model = Model::new(&app, frp.clone_ref());
        let scene = app.display.scene();
        let styles = StyleWatchFrp::new(&scene.style_sheet);
        let network = &frp.network;
        let logger = &model.logger;
        let frp_clone = frp.clone();
        let model_clone = model.clone_ref();
        frp::extend! { network
            init <- source_();

            initialization_finished <- toggle(&init);
            projects_need_update <- gate(&frp.projects_list, &initialization_finished);

            eval projects_need_update([logger] (list) {
                if let Err(err) = model_clone.update_projects_list(&list, frp_clone.clone_ref()) {
                    error!(logger, "Unable to update projects_list: {err}");
                }
            });

            frp.source.opened_project <+ frp.open_project.map(|name| Some(name.clone()));

            let shape  = app.display.scene().shape();
            position <- map(shape, |scene_size| {
                let x = -scene_size.width / 2.0;
                let y =  scene_size.height / 2.0;
                Vector2(x, y)
            });
            eval position ((pos) model.display_object.set_position_xy(*pos));
        }
        init.emit(());

        Self { model, styles, frp }
    }
}

impl display::Object for View {
    fn display_object(&self) -> &display::object::Instance {
        &self.model.display_object
    }
}

impl application::command::FrpNetworkProvider for View {
    fn network(&self) -> &frp::Network {
        &self.frp.network
    }
}

impl application::View for View {
    fn label() -> &'static str {
        "WelcomeScreen"
    }

    fn new(app: &Application) -> Self {
        Self::new(app)
    }

    fn app(&self) -> &Application {
        &self.model.application
    }
}