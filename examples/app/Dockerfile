FROM alpine:3.20
RUN mkdir -p /www
COPY index.html /www/index.html
EXPOSE 8000
CMD ["httpd", "-f", "-v", "-p", "8000", "-h", "/www"]
