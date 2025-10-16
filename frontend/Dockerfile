FROM node:20-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm install
COPY . .
RUN npm run build

FROM nginx:1.25-alpine
COPY --from=builder /app/dist /usr/share/nginx/html
COPY public/service-worker.js /usr/share/nginx/html/service-worker.js
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
