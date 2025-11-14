# Cypherpunk Legacy - 密碼朋克傳承

一個融合密碼學教育與像素藝術的網頁遊戲。探索爺爺留下的數位遺產，學習密碼學的核心概念。

## 🎮 遊戲原型

### 已完成的實驗

1. **BIP39 助記詞認證系統** (`bip39-game-final.html`)
   - 6 個單詞的助記詞系統
   - 即時驗證與視覺反饋
   - 密鑰派生與數位簽章驗證

2. **像素藝術渲染測試** (`pixel-art-test.html`)
   - 16x16 像素精靈系統
   - CSS 渲染模式比較
   - 互動式縮放測試
   - 多種調色板選項

3. **加密/解密演示** (`encryption-demo.html`)
   - 多種加密演算法 (AES-256, 3DES, RC4)
   - 即時加密/解密操作
   - 視覺化流程展示
   - 效能指標

4. **爺爺的房間 - 完整遊戲原型** (`grandpa-room-prototype.html`) ⭐ **NEW**
   - 可玩的像素藝術遊戲
   - 鍵盤控制的角色移動
   - 互動式場景與物件
   - 整合 BIP39 認證系統
   - 場景轉場效果
   - 進入 Cryptopolis 數位世界

## 🚀 快速開始

### 方式 1: 直接打開 HTML 文件

1. 克隆此儲存庫
2. 用瀏覽器打開任何 `.html` 文件
3. 開始探索！

### 方式 2: 本地伺服器（推薦）

```bash
# 使用 Python
python -m http.server 8000

# 或使用 Node.js
npx http-server
```

然後在瀏覽器中訪問 `http://localhost:8000`

## 🎯 遊戲原型使用指南

### 爺爺的房間原型 (grandpa-room-prototype.html)

**控制方式:**
- **方向鍵** 或 **WASD**: 移動角色
- **E 鍵**: 與物件互動（當靠近電腦時）

**遊戲流程:**
1. 遊戲開始時，你在爺爺的房間中
2. 使用方向鍵探索房間
3. 走到電腦旁邊（會出現「按 E 鍵互動」提示）
4. 按下 E 鍵開啟電腦的認證界面
5. 輸入 6 個助記詞解鎖電腦
6. 認證成功後，畫面會淡出並轉場到 Cryptopolis
7. 歡迎來到數位世界！

**有效的助記詞:**
- `digital freedom privacy secure network future`
- `crypto rebel underground anonymous protest freedom`
- `cipher encrypt message secret digital revolution`

**提示:**
- 探索房間中的細節：書架、窗戶、地毯
- 角色有簡單的碰撞檢測，無法穿過桌子
- 靠近電腦時，螢幕會發光

## 🛠️ 技術架構

### 核心技術
- **純 HTML/CSS/JavaScript** - 無需框架
- **Canvas API** - 2D 遊戲渲染
- **BIP39 庫** - 真實的助記詞驗證
- **CryptoJS** - 加密演算法

### 視覺風格
- **解析度**: 16x16 像素精靈，縮放至 64x64 (4x)
- **調色板**: 三色灰階配藍灰色強調
- **渲染**: CSS `image-rendering: pixelated`

### 遊戲引擎
- 60 FPS 遊戲循環
- 狀態機場景管理
- 事件驅動互動系統
- 平滑轉場效果

## 📁 檔案結構

```
cypherpunk-legacy/
├── bip39-game-final.html          # BIP39 認證實驗
├── pixel-art-test.html            # 像素藝術測試
├── encryption-demo.html           # 加密演示
├── grandpa-room-prototype.html    # 完整遊戲原型 ⭐
├── EXPERIMENT_SUMMARY.md          # 實驗總結
└── README.md                      # 本文件
```

## 🎨 遊戲特色

### 敘事設計
遊戲透過「爺爺的遺產」這個故事框架，將密碼學概念自然地融入遊戲體驗：
- 爺爺是一位密碼朋克先驅
- 他留下了一台需要助記詞解鎖的電腦
- 電腦是通往 Cryptopolis 數位世界的入口
- 玩家通過解謎學習密碼學知識

### 視覺風格
- 復古像素藝術風格
- 賽博朋克色調（深色背景、藍灰色調）
- 簡約但有細節的場景設計
- 平滑的動畫過渡

### 教育價值
- 實際使用 BIP39 助記詞系統
- 體驗密鑰派生過程
- 理解認證與授權概念
- 探索密碼學在數位世界中的應用

## 🔮 未來計劃

### 近期目標
- [ ] 添加更多房間和場景
- [ ] 實現 NPC 互動系統
- [ ] 擴展 Cryptopolis 世界
- [ ] 加入音效和背景音樂
- [ ] 添加更多密碼學謎題

### 中期目標
- [ ] 存檔/讀檔系統
- [ ] 多個角色精靈動畫
- [ ] 進階光照效果
- [ ] 任務與成就系統
- [ ] 密碼學知識庫

### 長期願景
- [ ] 多人協作模式
- [ ] 創建自己的密碼學謎題
- [ ] 與真實區塊鏈互動
- [ ] 社群驅動的內容
- [ ] 教育版本與課程整合

## 📚 學習資源

### 密碼學基礎
- [BIP39 規範](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki)
- [密碼學入門](https://www.coursera.org/learn/crypto)
- [密碼朋克宣言](https://www.activism.net/cypherpunk/manifesto.html)

### 遊戲開發
- [Canvas API 文件](https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API)
- [像素藝術教程](https://www.pixilart.com/tutorials)
- [遊戲設計模式](https://gameprogrammingpatterns.com/)

## 🤝 貢獻

歡迎貢獻！這個專案仍在早期開發階段。

### 貢獻方式
1. Fork 此儲存庫
2. 創建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 開啟 Pull Request

### 貢獻領域
- 🎨 像素藝術與視覺設計
- 🎮 遊戲機制與關卡設計
- 🔐 密碼學謎題與教育內容
- 🎵 音效與音樂
- 📖 文檔與教程
- 🐛 Bug 修復與優化

## 📜 授權

MIT License - 自由使用、修改和分享

## 🙏 致謝

- 密碼朋克運動的先驅們
- BIP39 與 CryptoJS 的開發者
- 所有熱愛自由軟體與隱私的人們

## 📞 聯繫

有問題或建議？歡迎開 Issue 或 Pull Request！

---

**Remember: Privacy is not a crime. Encryption is a human right.** 🔐✨
