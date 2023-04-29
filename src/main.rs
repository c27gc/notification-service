// use actix_web::{web, App, HttpResponse, HttpServer, Responder};
// use serde::{Deserialize, Serialize};
// use tokio::sync::mpsc;
// use tokio::time::{sleep, Duration};
// use uuid::Uuid;

// #[derive(Serialize, Deserialize)]
// struct NotificationRequest {
//     fcm_token: String,
//     message: String,
//     delay: u64,
// }

// #[derive(Serialize)]
// struct ResponseMessage {
//     message: String,
// }

// async fn send_notification(
//     data: web::Data<mpsc::Sender<(NotificationRequest, Uuid)>>,
//     notification_request: web::Json<NotificationRequest>,
// ) -> impl Responder {
//     let id = Uuid::new_v4();
//     let _ = data.get_ref().send((notification_request.into_inner(), id)).await;
//     HttpResponse::Ok().json(ResponseMessage {
//         message: format!("Notification scheduled with ID: {}", id),
//     })
// }

// async fn notification_scheduler(mut receiver: mpsc::Receiver<(NotificationRequest, Uuid)>) {
//     while let Some((notification_request, id)) = receiver.recv().await {
//         let delay = Duration::from_secs(notification_request.delay);
//         let message = notification_request.message.clone();
//         let fcm_token = notification_request.fcm_token.clone();

//         tokio::spawn(async move {
//             sleep(delay).await;
//             println!(
//                 "Notification with ID: {} sent to FCM token: {}, message: {}",
//                 id, fcm_token, message
//             );
//         });
//     }
// }

// #[actix_web::main]
// async fn main() -> std::io::Result<()> {
//     let (tx, rx) = mpsc::channel(32);
//     tokio::spawn(notification_scheduler(rx));

//     HttpServer::new(move || {
//         App::new()
//             .app_data(web::Data::new(tx.clone()))
//             .route("/send_notification", web::post().to(send_notification))
//     })
//     .bind("127.0.0.1:8080")?
//     .run()
//     .await
// }

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use chrono::Local;
use fcm::{Client, NotificationBuilder, MessageBuilder};

#[derive(Serialize, Deserialize, Clone)]
struct NotificationRequest {
    fcm_token: String,
    message: String,
    delay: u64,
}

#[derive(Serialize)]
struct ResponseMessage {
    message: String,
}

async fn send_notification(
    data: web::Data<Arc<Mutex<Vec<(NotificationRequest, Uuid)>>>>,
    notification_request: web::Json<Vec<NotificationRequest>>,
) -> impl Responder {
    let notifications: Vec<(NotificationRequest, Uuid)> = notification_request
        .into_inner()
        .into_iter()
        .map(|req| (req, Uuid::new_v4()))
        .collect();

    let mut list = data.lock().unwrap();
    list.extend(notifications.iter().cloned());

    HttpResponse::Ok().json(ResponseMessage {
        message: format!("{} notifications scheduled", notifications.len()),
    })
}

async fn notification_scheduler(
    data: web::Data<Arc<Mutex<Vec<(NotificationRequest, Uuid)>>>>,
) {
    loop {
        {
            let mut list = data.lock().unwrap();
            let mut i = 0;
            while i < list.len() {
                let (notification_request, id) = &list[i];
                let delay = Duration::from_secs(notification_request.delay);
                let message = notification_request.message.clone();
                let fcm_token = notification_request.fcm_token.clone();

                let id = *id;
                let fcm_token = fcm_token.clone();
                let message = message.clone();

                tokio::spawn(async move {
                    sleep(delay).await;
                    let now = chrono::Local::now().format("%H:%M:%S");
                    println!(
                        "[{}] Notification with ID: {} sent to FCM token: {}, message: {}",
                        now, id, fcm_token, message
                    );
                });

                i += 1;
            }

            list.truncate(0);
        }

        sleep(Duration::from_secs(1)).await;
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let notifications = Arc::new(Mutex::new(Vec::new()));

    tokio::spawn(notification_scheduler(
        web::Data::new(notifications.clone()),
    ));

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(notifications.clone()))
            .route("/send_notification", web::post().to(send_notification))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}