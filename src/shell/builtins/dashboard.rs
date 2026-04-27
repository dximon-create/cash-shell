use crate::store::Store;
use crate::web::server::DashboardServer;

pub fn run(store: &Store) {
    let cash_dir = store.db_path("history.db")
        .parent()
        .unwrap()
        .to_path_buf();
    let server = DashboardServer::new(cash_dir, 8080);
    server.start();
}
