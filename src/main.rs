mod engine;

use std::{cell::RefCell, error::Error, rc::Rc};

use engine::Engine;
use gio::prelude::*;
use glib::{self, clone};
#[allow(unused_imports)]
use log::*;
use unirun_if::{
    package::{Command, Hit, Package, PackageId, Payload},
    socket::{connection, stream_read_future, stream_write_future},
};

const SHOW_ON_EMPTY: bool = false;

async fn send_data(
    hits: Rc<RefCell<Vec<Hit>>>,
    connection: gio::SocketConnection,
) -> Result<(), Box<dyn Error>> {
    'send_data: for hit in hits.borrow().iter() {
        'send_hit: loop {
            let hit_package = Package::new(Payload::Hit(hit.clone()));
            let hit_package_id = hit_package.get_id();

            debug!("Sending {}", hit);
            stream_write_future(&connection.output_stream(), hit_package).await?;

            let response = stream_read_future(&connection.input_stream()).await?;
            debug!("Got response: {:?}", response);

            match response.payload {
                Payload::Result(result) => {
                    if let (response_id, Ok(())) = result {
                        if response_id == hit_package_id {
                            break 'send_hit;
                        }
                    }
                }
                Payload::Command(Command::Abort) => {
                    break 'send_data;
                }
                _ => unreachable!(),
            }
        }
    }
    Ok(())
}

async fn handle_command(
    command: &Command,
    id_to_answer: PackageId,
    connection: gio::SocketConnection,
    engines: Rc<RefCell<Vec<Engine>>>,
    hits: Rc<RefCell<Vec<Hit>>>,
    main_loop: glib::MainLoop,
) -> Result<(), Box<dyn Error>> {
    fn refresh_data(text: &str, engines: Rc<RefCell<Vec<Engine>>>, hits: Rc<RefCell<Vec<Hit>>>) {
        *engines.borrow_mut() = (if text.is_empty() && !SHOW_ON_EMPTY {
            Vec::new()
        } else {
            Engine::all()
        })
        .into_iter()
        .map(|e| e.with_query(text))
        .collect();
        *hits.borrow_mut() = engines.borrow().iter().map(Hit::from).collect();
    }

    match command {
        Command::Quit => {
            info!("Quit");

            let _ = stream_write_future(
                &connection.output_stream(),
                Package::new(Payload::Result((id_to_answer, Ok(())))),
            )
            .await;

            main_loop.quit();
        }
        Command::Abort => {}
        Command::GetData(text) => {
            // todo!();
            refresh_data(text, engines.clone(), hits.clone());

            stream_write_future(
                &connection.output_stream(),
                Package::new(Payload::Result((id_to_answer, Ok(())))),
            )
            .await?;

            send_data(hits.clone(), connection.clone()).await?;

            stream_write_future(
                &connection.output_stream(),
                Package::new(Payload::Command(Command::Abort)),
            )
            .await?;
        }
        Command::Activate(hit_id) => {
            if let Some(engine) = engines
                .borrow()
                .iter()
                .zip(hits.borrow().iter())
                .find_map(|(e, h)| if h.id == *hit_id { Some(e) } else { None })
            {
                stream_write_future(
                    &connection.output_stream(),
                    Package::new(Payload::Result((id_to_answer, {
                        let answer = match open::that(engine.build()) {
                            Ok(_) => Ok(()),
                            Err(e) => Err(format!("{}", e)),
                        };
                        debug!("Sending: {:?}", answer);
                        answer
                    }))),
                )
                .await?;
            } else {
                stream_write_future(
                    &connection.output_stream(),
                    Package::new(Payload::Result((
                        id_to_answer,
                        Err("Plugin info: cannot find data by Hit".to_owned()),
                    ))),
                )
                .await?;
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), glib::Error> {
    env_logger::init();

    let connection = connection()?;
    let engines = Rc::new(RefCell::new(Vec::new()));
    let hits = Rc::new(RefCell::new(Vec::new()));
    let main_loop = glib::MainLoop::new(None, true);

    let var_name = clone!(
        #[strong]
        main_loop,
        async move {
            loop {
                debug!("Waiting for command");

                let command_package = stream_read_future(&connection.input_stream())
                    .await
                    .unwrap_or_else(|e| {
                        error!("{}", e);
                        main_loop.quit();
                        panic!("{}", e)
                    });
                let command_package_id = command_package.get_id();
                debug!("Received: {:?}", command_package);

                match &command_package.payload {
                    Payload::Command(command) => {
                        handle_command(
                            command,
                            command_package_id,
                            connection.clone(),
                            engines.clone(),
                            hits.clone(),
                            main_loop.clone(),
                        )
                        .await
                        .unwrap_or_else(|e| {
                            error!("{}", e);
                            main_loop.quit();
                            panic!("{}", e)
                        });
                    }
                    _ => {
                        unreachable!("How to handle this: {:?}?", command_package)
                    }
                }
            }
        }
    );
    glib::spawn_future_local(var_name);

    main_loop.run();
    Ok(())
}
