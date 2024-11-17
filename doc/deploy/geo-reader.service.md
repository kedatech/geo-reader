[Unit]
Description=API en Rust
After=network.target

[Service]
Type=simple
ExecStart=/var/www/geo-reader/target/release/geo-reader
Restart=on-failure
WorkingDirectory=/var/www/geo-reader
User=root
Environment=RUST_LOG=info  # Activa los logs (opcional)

[Install]
WantedBy=multi-user.target

