# SilentNode Web — React + TypeScript + 3D Graph

## Quraşdırma

```bash
cd web
npm install
```

## İşlətmək

**1. API serveri başlat (Rust backend):**
```bash
cargo run -- api 3030
```

**2. Web dev serveri başlat:**
```bash
cd web
npm run dev
# → http://localhost:5173
```

## Build (production)
```bash
cd web
npm run build
# dist/ qovluğuna çıxır
```

## Xüsusiyyətlər

- **3D Force Graph** — Three.js üzərində, fizika simulasiyası ilə
- **Canlı data** — hər 10s-də API-dən avtomatik yenilənir
- **Node detail panel** — klik edib tam məlumat gör
- **Journal** — oxu + yeni qeyd əlavə et
- **Sidebar** — Season, Oracle siqnalları, Civilizations, Health
- **Klaviş qısa yolları:**
  - `N` = yeni node əlavə et
  - `G` = Graph tab-ına keç
  - `J` = Journal tab-ına keç
  - `R` = məlumatları yenilə
  - `Esc` = seçimi ləğv et

## API endpoint-ləri

Rust backend (port 3030) tam API:
- `GET  /nodes` — bütün node-lar
- `GET  /edges` — bütün əlaqələr
- `POST /thought` — yeni düşüncə materiallaşdır
- `POST /focus` — fokus hadisəsi qeydə al
- `GET  /journal` / `POST /journal`
- `GET  /oracle` — Oracle siqnalları
- `GET  /season` — Cognitive Season
- `GET  /civilizations`
- `GET  /analytics` — qraf sağlamlığı
- `GET  /analytics/pagerank`
- `GET  /dashboard` — standalone HTML dashboard
