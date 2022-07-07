use rand::Rng;

pub fn gen_random_filename() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
    abcdefghijklmnopqrstuvwxyz\
    0123456789";

    const FILENAME_LEN: usize = 24;
    let mut rng = rand::thread_rng();

    (0..FILENAME_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect::<String>()
}
