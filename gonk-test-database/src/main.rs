use gonk_test_database::*;

fn main() {
    init();

    bench(|| {
        let _item = names_from_album("Xen");
        // let par_item = db.par_names_from_album("Xen");
        // dbg!(item, par_item);
        // let item = db.par_artists();
    });
}
