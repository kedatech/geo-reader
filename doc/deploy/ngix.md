## Ngix configuracion
/etc/nginx/sites-available/geo-reader

server {
    listen 80;
    server_name geo-reader.ch1vo.com

    location /api/ {  # Redirige solicitudes que comienzan con /api/
        proxy_pass http://127.0.0.1:8087;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }
}

