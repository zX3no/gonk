use gonk_test_database::*;

fn main() {
    let db = Database::new();

    bench(|| {
        let _item = db.names_from_album("Xen");
        // let par_item = db.par_names_from_album("Xen");
        // dbg!(item, par_item);
        // let item = db.par_artists();
    });
}
