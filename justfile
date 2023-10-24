set shell := ["bash", "-uc"]

doc:
  cargo doc
  simple-http-server -i -o target/doc
install:
  RUSTFLAGS="-C target-cpu=native" cargo build --release 
  - mkdir /srv/school-mining
  cp target/release/school-mining-scraper /srv/school-mining/
prepare:
  - sudo groupadd admin
  sudo usermod -a -G admin $USER
  - sudo mkdir /srv
  sudo chown root:admin /srv
  sudo chmod 775 /srv
  sudo chmod g+s /srv

 