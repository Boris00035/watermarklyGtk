// Todo: 
// watermark knop
// remove explicit Rc's, as gtk implements that itself and can just do .clone()

use adw::{
    prelude::*,
    Application,
    glib,
    ApplicationWindow, 
    gdk::Rectangle,
    gdk::Texture,
    ToggleGroup,
    Toggle,
    HeaderBar,
    NavigationPage,
    OverlaySplitView,
    PreferencesGroup,
    ActionRow,
    Spinner,
    SpinRow
};

use gtk::{
    Align, 
    Box, 
    Orientation, 
    Button,
    Scale,
    Adjustment,
    FileDialog,
    Grid,
    ScrolledWindow,
    Overlay,
    Picture,
    gdk_pixbuf::{Pixbuf, PixbufRotation, InterpType},
    Stack,
    StackTransitionType,
    Entry,
    EntryBuffer,
    PositionType,
    ProgressBar,
    gio,
    };

use std::{
    cell::RefCell, fs::{self, File}, io, path::PathBuf, rc::Rc, char,
};

use rayon::prelude::*;

use exif;
use rand::prelude::IndexedRandom;



const APP_ID: &str = "org.gtk_rs.Cliquemark";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    return app.run();
}

fn calculate_watermark_position(
    preview_image_dimensions:       &RefCell<[i32; 2]>,
    preview_watermark_dimensions:   &RefCell<[i32; 2]>,
    image_preview:                  &Rc<Picture>,
    scale_slider_value:             &f64,
    margin_value:                   i32,
    active_alignment_array:         [i32; 4],
) -> Rectangle 
{
    let mut width_ratio = 0.0;
    let mut height_ratio = 0.0;
    let mut global_scale = 1.0;
    
    if preview_image_dimensions.borrow()[0] != 0 && preview_image_dimensions.borrow()[1] != 0 {
        width_ratio = preview_watermark_dimensions.borrow()[0] as f64 / preview_image_dimensions.borrow()[0] as f64;
        height_ratio = preview_watermark_dimensions.borrow()[1] as f64 / preview_image_dimensions.borrow()[1] as f64;
    
        global_scale = image_preview.width() as f32 / preview_image_dimensions.borrow()[0] as f32;
    }


    let width = (width_ratio * image_preview.width() as f64 * scale_slider_value).ceil() as i32;
    let height = (height_ratio * image_preview.height() as f64 * scale_slider_value).ceil() as i32;

    let adjusted_margin = (margin_value as f32 * global_scale).ceil() as i32;

    let x = (active_alignment_array[1] + active_alignment_array[3]) * (image_preview.width() - width - adjusted_margin)
                + (active_alignment_array[0] + active_alignment_array[2]) * adjusted_margin;
    let y = (active_alignment_array[2] + active_alignment_array[3]) * (image_preview.height() - height - adjusted_margin)
                + (active_alignment_array[0] + active_alignment_array[1]) * adjusted_margin;

    return Rectangle::new(x, y, width, height);
}


fn build_ui(app: &Application) {
    let main_page_splitview = OverlaySplitView::builder()
        .min_sidebar_width(450.0)
        .build();

    let main_stack = Stack::builder()
        .transition_type(StackTransitionType::Crossfade)
        // .interpolate_size(true)
        .vhomogeneous(true)
        .hhomogeneous(true)
        .build();   
    main_stack.add_named(&main_page_splitview, Some("main_page"));

    // Create a window
    let main_window = Rc::new(ApplicationWindow::builder()
        .application(app)
        .title("Cliquemark")
        .content(&main_stack)
        .build()
    );
    // main_window.set_size_request(800, 200);
    main_window.set_default_size(1000, 600);

    let settings_header_container = Box::builder()
        .orientation(Orientation::Vertical)
        .vexpand(true)
        .build();

    let settings_header = HeaderBar::builder()
        // .margin_bottom(10)
        .build();
    settings_header.add_css_class("flat");
    settings_header_container.append(&settings_header);
    
    let settings_box_container = Box::builder()
        .vexpand(true)
        .build();
    settings_header_container.append(&settings_box_container);

    // settings container 
    let settings_box = Box::builder()
        .valign(Align::Center)
        .halign(Align::Center)
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(50)
        .margin_bottom(50)
        .build();
    settings_box_container.append(&settings_box);

    let settings_sidebar = NavigationPage::builder()
        .child(&settings_header_container)
        .title("Settings")
        .vexpand(true)
        .build();
    main_page_splitview.set_sidebar(Some(&settings_sidebar));
    
    // Get rid of this grid, replace with nested gtkBox
    let selection_button_grid = Grid::builder()
        .margin_bottom(40)
        .build();

    selection_button_grid.set_row_spacing(12);
    selection_button_grid.set_column_spacing(12);    
    settings_box.append(&selection_button_grid);

    // folder directory chooser
    let choose_folder_button = Button::builder()
        .label("Select Folder")
        // .hexpand(false)
        // .vexpand(true)
        .valign(Align::Fill)
        .halign(Align::Fill)
        .build();

    let chosen_folder_text = Rc::new(Entry::builder()
        .hexpand(true)
        .vexpand(false)
        .editable(false)
        .sensitive(false)
        .build()
    );
    let default_entry_text = EntryBuffer::new(Some("Nothing chosen"));
    chosen_folder_text.set_buffer(&default_entry_text);

    let folder_scrolled_container = ScrolledWindow::builder()
        .build();

    // Add the TextView to the ScrolledWindow
    folder_scrolled_container.set_child(Some(&*chosen_folder_text));

    selection_button_grid.attach(&choose_folder_button, 0, 0, 1, 1);
    selection_button_grid.attach(&folder_scrolled_container, 1,0,1,1);

    // watermark chooser
    let choose_watermark_button = Button::builder()
        .label("Select Watermark")
        // .hexpand(false)
        // .vexpand(true)
        .valign(Align::Fill)
        .halign(Align::Fill)
        .build();
    // choose_watermark_button.add_css_class("suggested-action");
    
    let chosen_watermark_text = Rc::new(Entry::builder()
        .hexpand(true)
        .vexpand(false)
        .editable(false)
        .sensitive(false)
        .build()
    );
    let default_watermark_text = EntryBuffer::new(Some("Nothing chosen"));
    chosen_watermark_text.set_buffer(&default_watermark_text);

    let watermark_scrolled_container = ScrolledWindow::builder()
        .build();
    watermark_scrolled_container.set_child(Some(&*chosen_watermark_text));
    selection_button_grid.attach(&choose_watermark_button, 0, 1, 1, 1);
    selection_button_grid.attach(&watermark_scrolled_container, 1,1,1,1);


    // Alignment check boxes
    let top_left_toggle = Toggle::builder()
        .label("Top left")
        .build();
    let top_right_toggle = Toggle::builder()
        .label("Top right")
        .build();
    let bottom_left_toggle = Toggle::builder()
        .label("Bottom left")
        .build();
    let bottom_right_toggle = Toggle::builder()
        .label("Bottom right")
        .build();
    // top_left_toggle.set_child(true);
    let alignment_toggle_group = Rc::new(ToggleGroup::builder()
        .valign(Align::Center)
        .build()
    );
    alignment_toggle_group.add(top_left_toggle);
    alignment_toggle_group.add(top_right_toggle);
    alignment_toggle_group.add(bottom_left_toggle);
    alignment_toggle_group.add(bottom_right_toggle);
    alignment_toggle_group.set_active(3);
    
    settings_box.append(&*alignment_toggle_group);


    let image_configs_container = PreferencesGroup::builder()
    // .can_focus(false)
    .build();
    settings_box.append(&image_configs_container);


    // scale slider
    let scale_adjustment = Adjustment::new(1.0,0.01, 2.0,0.01,0.01,0.01); 
    let scale_slider = Rc::new(Scale::builder()
        .digits(2)
        .hexpand(true)
        .draw_value(true)
        .adjustment(&scale_adjustment)
        // .margin_bottom(15)    
        .width_request(250)
        .value_pos(PositionType::Right)
        .build()
    );

    let settings_action_row = ActionRow::builder()
        .title("Scale:")
        .build();
    settings_action_row.add_suffix(&*scale_slider);
    image_configs_container.add(&settings_action_row);


    let margin_adjustment = Adjustment::new(0.0, 0.0, 1000.0, 10.0, 10.0, 0.0);
    let margin_spin_row = Rc::new(SpinRow::builder()
        .title("Margin:")
        .adjustment(&margin_adjustment)
        // .climb_rate(10.0)
        .build()
    );
    image_configs_container.add(&*margin_spin_row);

    
    // confirm button
    let confirm_button = Button::builder()
        .halign(Align::Center)
        .label("Watermark")
        .margin_top(70)
        .build();
    confirm_button.add_css_class("suggested-action");
    confirm_button.add_css_class("pill");
    settings_box.append(&confirm_button);


    let preview_header = HeaderBar::builder()
        .build();
    preview_header.add_css_class("flat");

    
    let header_container = Box::builder()
        .orientation(Orientation::Vertical)
        .build(); 
    header_container.append(&preview_header);

    let preview_navigation_page = NavigationPage::builder()
        .title("Preview")
        .child(&header_container)
        .build();
    main_page_splitview.set_content(Some(&preview_navigation_page));

    let preview_side_box = Box::builder()
        .margin_start(50)
        .margin_end(50)
        // .margin_top(50)
        .margin_bottom(50)
        .orientation(Orientation::Vertical)
        .halign(Align::Center)
        .valign(Align::Fill)
        .hexpand(true)
        .vexpand(true)
        .build();
    header_container.append(&preview_side_box);  
      
    let preview_side_sub_box = Box::builder()
        .orientation(Orientation::Vertical)
        .halign(Align::Fill)
        .valign(Align::Center)
        .hexpand(true)
        .vexpand(true)
        .build();
    preview_side_box.append(&preview_side_sub_box);

    let preview_widget = Rc::new(Overlay::builder()
        .margin_top(10)
        .build()
    );
    preview_side_sub_box.append(&*preview_widget);

    let image_preview = Rc::new(Picture::builder()
        .build()
    );
    preview_widget.set_child(Some(&*image_preview));

    let watermark_preview = Rc::new(Picture::builder()
        .build()
    );
    
    preview_widget.add_overlay(&*watermark_preview);
    
    alignment_toggle_group.connect_active_notify({
        let preview_widget = Rc::clone(&preview_widget);
        move |_| {
            let _ = &preview_widget.queue_allocate();
        }
    });

    let preview_image_dimensions: Rc<RefCell<[i32; 2]>> = Rc::new(RefCell::new([0, 0]));
    let preview_watermark_dimensions: Rc<RefCell<[i32; 2]>> = Rc::new(RefCell::new([0, 0]));

    preview_widget.connect_get_child_position(
    {
        let preview_image_dimensions = Rc::clone(&preview_image_dimensions);
        let preview_watermark_dimensions = Rc::clone(&preview_watermark_dimensions);
        let image_preview = Rc::clone(&image_preview);
        let scale_slider = Rc::clone(&scale_slider);
        let margin_input = Rc::clone(&margin_spin_row);


        let alignment_toggle_group = Rc::clone(&alignment_toggle_group);

        move |_, _watermark_preview| {
            let alignment_config_array: [i32; 4] = match alignment_toggle_group.active() {
                0 => [1, 0, 0, 0],
                1 => [0, 1, 0, 0],
                2 => [0, 0, 1, 0],
                3 => [0, 0, 0, 1],
                _ => [0, 0, 0, 1],
            };

            let watermark_rectangle = calculate_watermark_position(
                &preview_image_dimensions,
                &preview_watermark_dimensions,
                &image_preview,
                &scale_slider.value(),
                margin_input.value() as i32,
                alignment_config_array,
            );
            return Some(watermark_rectangle);
        }
    });    

    scale_slider.connect_value_changed({
        let preview_widget = Rc::clone(&preview_widget);        
        move |_| {
            let _ = &preview_widget.queue_allocate();
        }
    });

    margin_spin_row.connect_value_notify({
        let preview_widget = Rc::clone(&preview_widget);        
        move |_| {
            let _ = &preview_widget.queue_allocate();
        }
    });


    let loader_header_container = Box::builder()
        .orientation(Orientation::Vertical)
        .build();

    let loader_navigation_page = NavigationPage::builder()
    .child(&loader_header_container)
    .title("Cliquemark")    
    .build();

    let loader_header = HeaderBar::builder()
        .build();
    loader_header.add_css_class("flat");
    loader_header_container.append(&loader_header);

    let loader_page_container = Box::builder()
    .orientation(Orientation::Vertical)
    .valign(Align::Center)
    .halign(Align::Center)
    .vexpand(true)
    .build();
    loader_header_container.append(&loader_page_container);
    
    main_stack.add_named(&loader_navigation_page, Some("loader_page"));

    let watermark_loading_spinner = Spinner::builder()
        .height_request(50)
        .build();
    loader_page_container.append(&watermark_loading_spinner);

    let watermark_progress_bar = Rc::new(ProgressBar::builder()
        .width_request(300)
        .margin_top(30)
        .show_text(true)
        .build()
    );
    loader_page_container.append(&*watermark_progress_bar);

    // let cancel_button = Button::builder()
    //     .halign(Align::Center)
    //     .label("Cancel")
    //     .margin_top(70)
    //     .build();
    // cancel_button.add_css_class("destructive-action");
    // cancel_button.add_css_class("pill");
    // loader_page_container.append(&cancel_button);

    

    choose_folder_button.connect_clicked({
        let main_window = Rc::clone(&main_window);
        let watermark_progress_bar = Rc::clone(&watermark_progress_bar);
        let chosen_folder_text= Rc::clone(&chosen_folder_text);

        let preview_image_dimensions = Rc::clone(&preview_image_dimensions);

        move |_| {
            let folder_dialog = FileDialog::builder()
            .title("Select Folder")
            .build();

            let chosen_folder_text = Rc::clone(&chosen_folder_text);            
            let image_preview = Rc::clone(&image_preview);
            let watermark_progress_bar = Rc::clone(&watermark_progress_bar);
            let preview_image_dimensions = Rc::clone(&preview_image_dimensions);

            folder_dialog.select_folder(Some(&*main_window),None::<&gtk::gio::Cancellable>, 
            move |result| {
                let folder_path: PathBuf;
                if let Ok(path) = result {
                    folder_path = path.path().unwrap();
                } else {
                    return;
                }
                
                chosen_folder_text.set_text(folder_path.to_str().unwrap());
                
                let random_preview_entry;
                let entries = std::fs::read_dir(folder_path).unwrap()
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    if is_image_file(&path) {
                        Some(path)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
                
                if entries.is_empty() {
                    println!("No image files found in the folder.");
                    return;
                }

                watermark_progress_bar.set_pulse_step(1.0 / entries.len() as f64);

                let mut rng = rand::rng();
                if let Some(random_image) = entries.choose(&mut rng) {
                    random_preview_entry = random_image;
                } else {
                    todo!();
                };
                
                let mut preview_image_pixbuf = Pixbuf::from_file(&random_preview_entry).unwrap();

                preview_image_pixbuf = match random_preview_entry.extension().and_then(|ext| ext.to_str()) {
                    Some("png") => preview_image_pixbuf,
                    Some("PNG") => preview_image_pixbuf,
                    _ => apply_exif_rotation(random_preview_entry, preview_image_pixbuf),
                };

                let mut image_preview_dims = preview_image_dimensions.borrow_mut();

                image_preview_dims[0] = preview_image_pixbuf.width(); 
                image_preview_dims[1] = preview_image_pixbuf.height();  
                
                image_preview.set_paintable( Some(&Texture::for_pixbuf(&preview_image_pixbuf)) );
                  
            });
        }
    });

    choose_watermark_button.connect_clicked(
        {
        let main_window = Rc::clone(&main_window);
        let chosen_watermark_text = Rc::clone(&chosen_watermark_text);
        let watermark_preview = Rc::clone(&watermark_preview);   

        move |_| {
            let file_dialog = FileDialog::builder()
            .title("Select Watermark")
            .build();

            let chosen_watermark_text = Rc::clone(&chosen_watermark_text); 
            let watermark_preview = Rc::clone(&watermark_preview);   
            let preview_watermark_dimensions = Rc::clone(&preview_watermark_dimensions);
            let preview_widget = Rc::clone(&preview_widget);
         
            file_dialog.open(Some(&*main_window),None::<&gtk::gio::Cancellable>, move |result| {
                match result {
                    Ok(file) => {
                        let file_path = &file.path().unwrap();
                        // let file_path: &std::path::Path = &file.path().unwrap();
                        chosen_watermark_text.set_text(&file_path.to_str().unwrap());
                        
                        let mut preview_watermark_pixbuf = Pixbuf::from_file(&file_path).unwrap();

                        let mut watermark_preview_dims = preview_watermark_dimensions.borrow_mut();
                        watermark_preview_dims[0] = preview_watermark_pixbuf.width(); 
                        watermark_preview_dims[1] = preview_watermark_pixbuf.height();  
                        // watermark_preview.set_file( Some(&File::for_path(&file_path)) );

                        preview_watermark_pixbuf = match file_path.extension().and_then(|ext| ext.to_str()) {
                            Some("png") => preview_watermark_pixbuf,
                            Some("PNG") => preview_watermark_pixbuf,
                            _ => apply_exif_rotation(file_path, preview_watermark_pixbuf),
                        };
                        watermark_preview.set_paintable( Some(&Texture::for_pixbuf(&preview_watermark_pixbuf)) );
                    }
                    Err(error) => {
                        println!("Error: {}", error);
                    }
                }
            });
            let _ = &preview_widget.queue_allocate();
        }
    });

    
    let (watermarking_state_sender, watermarking_state_receiver) = async_channel::bounded(1);
    let (progress_sender, progress_receiver) = async_channel::bounded(1);

    confirm_button.connect_clicked(move |_| {
        let chosen_folder = (&chosen_folder_text).text().to_string();
        let chosen_watermark = (&chosen_watermark_text).text().to_string();
        let scale = (&scale_slider).value();
        let margin = (&margin_spin_row).value() as i32;
        let alignment = match &alignment_toggle_group.active() {
            0 => [1, 0, 0, 0],
            1 => [0, 1, 0, 0],
            2 => [0, 0, 1, 0],
            3 => [0, 0, 0, 1],
            _ => [0, 0, 0, 1],
        };

        let watermarking_state_sender = watermarking_state_sender.clone();
        let progress_sender = progress_sender.clone();

        gio::spawn_blocking({
            move || {
                apply_watermark(
                    chosen_folder,  
                    chosen_watermark,
                    scale,
                    margin,
                    alignment,
                    watermarking_state_sender,
                    progress_sender);
                }
            }
        );
    });

    // Queue the async block to update the stack_page
    glib::spawn_future_local(glib::clone!(
        #[weak]
        confirm_button,
        #[weak]
        main_stack,
        async move {
            while let Ok(state_bool) = watermarking_state_receiver.recv().await {
                confirm_button.set_sensitive(state_bool);
                let active_page_name = match state_bool {
                    true => "main_page",
                    false => "loader_page",
                };
                let _ = &main_stack.set_visible_child_full(active_page_name, gtk::StackTransitionType::Crossfade);
            }
        }
    ));

    // Queue the async block to update the progress_bar
    glib::spawn_future_local(glib::clone!(
        #[weak]
        watermark_progress_bar,
        async move {
            while let Ok(progress_value) = progress_receiver.recv().await {
                let progress_bar_value = (watermark_progress_bar.fraction() / watermark_progress_bar.pulse_step()).floor() as i32;

                if progress_bar_value != progress_value {
                    watermark_progress_bar.set_fraction((progress_value as f64) * watermark_progress_bar.pulse_step());
                };
            }
        }
    ));

    // cancel_button.connect_clicked(move |_| {
    //     let _ = &main_stack.set_visible_child_full("main_page", gtk::StackTransitionType::Crossfade);
    // });

    main_window.present();
}

fn apply_watermark( 
    chosen_folder:              String, 
    chosen_watermark:           String, 
    scale:                      f64,
    margin:                     i32,
    alignment:                  [i32; 4],
    watermarking_state_sender:  async_channel::Sender<bool>, 
    progress_sender:            async_channel::Sender<i32>) {    
    // TODO: SANITIZE INPUT BEFORE CALLING APPLY_WATERMARK
    
    watermarking_state_sender
        .send_blocking(false)
        .expect("The confirm channel needs to be open.");
    
    // println!("{:?}", chosen_folder);
    let path_buf = PathBuf::from(&chosen_folder); 
    
    let image_entries = match std::fs::read_dir(path_buf) {
        Ok(entries) => entries,
        Err(error_message) => {
            eprintln!("Failed to read directory: {}", error_message);
            watermarking_state_sender
            .send_blocking(true)
            .expect("The confirm channel needs to be open.");            
            return;
        }
    }.filter_map(|entry| {
        let entry = entry.ok()?;
        let path = entry.path();
        if is_image_file(&path) {
            return Some(path);
        } else {
            return None;
        }
    }).collect::<Vec<_>>();
    
    let mut watermark_pixbuf = Pixbuf::from_file(&chosen_watermark).map_err(|error| error.message().to_string()).unwrap();
    watermark_pixbuf = watermark_pixbuf.apply_embedded_orientation().unwrap();
    

    let mut target_parent = PathBuf::from(&chosen_folder);
    target_parent.push("../");
    let target_folder = create_target_folder(("watermarked").to_string(), target_parent).unwrap();
    

    let mut progress_value = 0;
    let _results_array: Vec<Result<PathBuf, String>> = image_entries.into_iter().map(|image_entry| {
        progress_value += 1;
        progress_sender.send_blocking(progress_value).expect("The progress channel needs to be open.");

        let mut image_pixbuf = Pixbuf::from_file(&image_entry).map_err(|error| error.message().to_string()).unwrap();
        image_pixbuf = image_pixbuf.apply_embedded_orientation().unwrap();


        let watermark_scaled_width = (watermark_pixbuf.width() as f64 * scale).ceil() as i32;
        let watermark_scaled_height = (watermark_pixbuf.height() as f64 * scale).ceil() as i32;

        let watermark_position_x = (alignment[1] + alignment[3]) * (&image_pixbuf.width() - watermark_scaled_width - margin)
                + (alignment[0] + alignment[2]) * margin;
        let watermark_position_y = (alignment[2] + alignment[3]) * (&image_pixbuf.height() - watermark_scaled_height - margin)
                + (alignment[0] + alignment[1]) * margin;

        // println!("x: {:?}, y: {:?}, watermark_size: {:?},{:?} align: {:?}", watermark_poition_x, watermark_position_y, watermark_scaled_width, watermark_scaled_height, alignment);
        watermark_pixbuf.composite(
            &mut image_pixbuf,
            watermark_position_x,
            watermark_position_y,
            watermark_scaled_width,
            watermark_scaled_height,
            watermark_position_x as f64,
            watermark_position_y as f64,
            1.0,
            1.0,
            InterpType::Bilinear,
            255
        );

        let _ = image_pixbuf.savev(target_folder.join(&image_entry.file_name().unwrap()), "png", &[]);
        return Ok(image_entry);
    }).collect();


    watermarking_state_sender
    .send_blocking(true)
    .expect("The confirm channel needs to be open.");
}


fn apply_exif_rotation(file_path: &std::path::Path, image_pixbuf: Pixbuf) -> Pixbuf {
    let file = File::open(file_path).unwrap();
    let mut bufreader = std::io::BufReader::new(&file);
    let exifreader = exif::Reader::new();
    let exif: exif::Exif = match exifreader.read_from_container(&mut bufreader) {
        Ok(exif) => exif,
        Err(e) => {
            eprintln!("Failed to read EXIF data: {}", e);
            return image_pixbuf;
        }
    };
    
    let image_orientation = match exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY) {
        Some(orientation) =>
            match orientation.value.get_uint(0) {
                Some(v @ 1..=8) => v,
                _ => {
                    eprintln!("Exif orientation value is broken, file:{:?}", file_path.to_str().unwrap());
                    1
                },
            },
        Option::None => 1,
    };

    let corrected_pixbuf = match image_orientation {
        1 => Some(image_pixbuf),
        2 => image_pixbuf.flip(true),
        3 => image_pixbuf.rotate_simple(PixbufRotation::Upsidedown),
        4 => image_pixbuf.flip(false),
        5 => image_pixbuf.flip(true).unwrap().rotate_simple(PixbufRotation::Counterclockwise),
        6 => image_pixbuf.rotate_simple(PixbufRotation::Clockwise),
        7 => image_pixbuf.flip(true).unwrap().rotate_simple(PixbufRotation::Clockwise),
        8 => image_pixbuf.rotate_simple(PixbufRotation::Counterclockwise),
        _ => {
            eprintln!{"The exif orientation value of file \"{}\" has a value higher outside of 1-8, which is unexpected, no rotation applied", file_path.to_str().unwrap()};
            Some(image_pixbuf)
            },
    }.expect("Failed to apply exif transformation");

    return corrected_pixbuf;
}

fn is_image_file(path: &std::path::Path) -> bool {
    if let Some(extension) = path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "avif" | "ico")
    } else {
        false
    }
}

fn create_target_folder(base_name: String, target_parent: PathBuf) -> Result<PathBuf, String> {
    
    let target_folder = target_parent.join(&base_name);
    // println!("{:?}", target_parent);
    if fs::create_dir(&target_folder).is_ok() {
        return Ok(target_folder);
    }
    
    let mut i = 1;
    
    loop {
        let mut buf = base_name.clone();

        buf.push(char::from_digit(i, 10).unwrap());

        println!("{:?}", target_parent);
        let target_folder = target_parent.join(&buf);
        println!("{:?}", target_folder);

        match fs::create_dir(&target_folder) {
            Ok(()) => return Ok(target_folder),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => i += 1,
            Err(_error) => return Err(("Failed to create directory").to_string()),
        }
    }
}