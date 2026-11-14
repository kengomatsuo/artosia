//! Made-up worst-case slides for the poster styles' scenes: the designer's sample data, no real church.

pub const CHURCH: &str = "Gereja Bethel Kasih Nusantara";
/// (day of November, time, event).
pub const ROWS: [(u8, &str, &str); 8] = [
    (15, "07.00", "Ibadah Raya I"),
    (15, "09.30", "Ibadah Raya II dan Sekolah Minggu"),
    (17, "18.30", "Persekutuan Doa dan Pemahaman Alkitab"),
    (18, "19.00", "Latihan Paduan Suara dan Tim Pemusik"),
    (19, "17.30", "Pelayanan Kunjungan Kasih ke Rumah Sakit"),
    (20, "19.00", "Ibadah Doa Puasa Bersama Seluruh Jemaat"),
    (21, "09.00", "Rapat Majelis dan Pengurus Komisi Anak"),
    (21, "16.00", "Bakti Sosial dan Pembagian Sembako Jemaat"),
];
pub const SPECIAL_ROW: usize = 5;
pub const NAMES: [(u8, &str); 30] = [
    (1, "Bpk. Daniel Hutagalung"),
    (2, "Ibu Martha Siregar"),
    (2, "Sdri. Elisabet Kurniawan Putri"),
    (3, "Adk. Jonathan Wijaya"),
    (4, "Bpk. Markus Tarigan"),
    (5, "Ny. Yuliana Sri Rahayu"),
    (6, "Sdr. Gabriel Manurung"),
    (7, "Ibu Lidia Pangemanan"),
    (8, "Bpk. Paulus Setiabudi Wibowo"),
    (9, "Adk. Keren Sembiring"),
    (10, "Ny. Debora Anastasia Nainggolan"),
    (11, "Bpk. Timotius Sinaga"),
    (12, "Sdri. Ruth Natalia Panjaitan"),
    (13, "Ibu Hana Kristiani"),
    (14, "Bpk. Yosafat Mandagi"),
    (15, "Sdr. Andreas Prasetyo Nugroho"),
    (15, "Adk. Grace Marpaung"),
    (16, "Ny. Magdalena Tumbelaka"),
    (17, "Bpk. Benyamin Lukas Sihombing"),
    (18, "Ibu Susi Handayani"),
    (18, "Sdri. Priskila Wulandari Saragih"),
    (19, "Adk. Samuel Gunawan"),
    (20, "Bpk. Rudi Hartono Silalahi"),
    (21, "Ny. Veronika Dewi Tampenawas"),
    (22, "Ibu Natalia Simbolon"),
    (23, "Sdr. Joshua Immanuel Purba"),
    (24, "Bpk. Hizkia Sutanto"),
    (26, "Adk. Abigail Siahaan"),
    (28, "Ny. Rosalinda Marbun"),
    (30, "Bpk. Thomas Agustinus Sitompul"),
];
pub const FUNDS: [(&str, i64); 6] = [
    ("Persembahan Syukur Ibadah Raya", 234567890123),
    ("Persepuluhan Jemaat", 45678901234),
    ("Pembangunan Gedung Gereja dan Pastori Tahap Dua", 1234567890),
    ("Diakonia dan Pelayanan Kasih Jemaat", 96300000),
    ("Sekolah Minggu dan Remaja", 12500000),
    ("Misi dan Penginjilan", 8450000),
];
pub const ACCOUNT: [&str; 4] = ["Bank Central Asia", "831-2745-906", "Yayasan Gereja Bethel Kasih Nusantara", "Tulis kode angka di akhir nomor: 5\u{a0}Minggu, 7\u{a0}Syukur, 8\u{a0}Perpuluhan, 10\u{a0}Natal.\nKonfirmasi ke bendahara lewat WhatsApp 0812-3456-7890."];
pub const REFERENCE: &str = "Yesaya 43:1–2";
pub const VERSES: [(u32, &str); 3] = [
    (1, "Beginilah firman TUHAN yang menciptakan engkau, hai Yakub: Janganlah takut, sebab Aku telah menebus engkau, Aku telah memanggil engkau dengan namamu, engkau ini kepunyaan-Ku."),
    (2, "Apabila engkau menyeberang melalui air, Aku akan menyertai engkau; apabila engkau berjalan melalui api, engkau tidak akan hangus, dan nyala api tidak akan menghanguskan engkau."),
    (3, "Sebab Aku ini TUHAN, Allahmu, Yang Mahakudus, Allah Israel, dan Juruselamatmu."),
];
pub const NOTICE_TITLE: &str = "Retret Keluarga Jemaat";
pub const NOTICE_LINES: [&str; 2] = ["Sabtu\u{a0}12\u{a0}–\u{a0}Minggu\u{a0}13\u{a0}Desember\u{a0}2026, Wisma\u{a0}Bukit\u{a0}Doa, Cipanas", "Pendaftaran di meja informasi sampai 6\u{a0}Desember"];
