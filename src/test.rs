use aes::Aes256; use cbc::Decryptor; fn f() { Decryptor::<Aes256>::new(Default::default(), Default::default()).decrypt_padded_vec_mut::<aes::cipher::block_padding::Pkcs7>(&[]); }
