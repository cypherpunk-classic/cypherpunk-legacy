# Taproot Explorer - 互動式學習遊戲

## 概述

Taproot Explorer 是一個互動式教育遊戲，旨在教授玩家關於 Bitcoin 的 BIP 340（Schnorr 簽名）和 BIP 341（Taproot 地址）的核心概念。遊戲採用密碼朋克風格設計，使用 Bitcoin 標誌性的橙色配色方案。

**訪問地址**: `http://localhost:8080/src/html/taproot-explorer.html`

## 技術架構

### 文件結構

```
src/
├── html/
│   └── taproot-explorer.html    # 主遊戲 HTML
├── css/
│   └── taproot-explorer.css     # 遊戲樣式
└── js/
    └── taproot-explorer.js      # 遊戲邏輯
```

### 設計原則

1. **模塊化設計**: HTML、CSS、JavaScript 完全分離
2. **漸進式解鎖**: 完成一個挑戰才能解鎖下一個
3. **即時反饋**: 每個操作都有視覺和分數反饋
4. **教育導向**: 每個挑戰都包含詳細的加密學原理解釋

## 遊戲挑戰

### 挑戰 1: Nonce 重用攻擊 (200 分)

**學習目標**: 理解 nonce 重用的災難性後果

#### 技術細節

**Schnorr 簽名方程**:
```
s = k + e·d (mod n)
```
其中:
- `k` = nonce（隨機數）
- `e` = challenge（挑戰值）= H(R || P || m)
- `d` = 私鑰
- `R` = k·G（nonce 的公鑰點）

**漏洞利用**:
當兩個簽名使用相同的 nonce 時:
```
s₁ = k + e₁·d
s₂ = k + e₂·d
```

通過減法消除 k:
```
s₁ - s₂ = (e₁ - e₂)·d
```

求解私鑰:
```
d = (s₁ - s₂) / (e₁ - e₂) mod n
```

#### 實現函數

**`generateLeakedData()`**
- 生成"中本聰"的私鑰和公鑰
- 創建兩個使用相同 nonce 的簽名
- 顯示洩露的數據（公鑰、消息、簽名）

**`calculatePrivateKey()`**
- 使用代數方法恢復私鑰
- 模擬 `d = (s₁ - s₂) / (e₁ - e₂)` 計算
- 顯示恢復的私鑰

**`forgeSignature()`**
- 使用恢復的私鑰創建偽造簽名
- 證明攻擊者可以代表受害者簽署任何消息

#### 挑戰流程

1. 點擊 "Generate Leaked Data" 生成洩露數據
2. 查看兩個使用相同 R 值的簽名
3. 點擊 "Calculate Private Key" 恢復私鑰
4. 輸入自定義消息並點擊 "Forge Signature" 偽造簽名
5. 點擊 "Complete Challenge" 獲得 200 分並解鎖下一關

### 挑戰 2: MuSig2 - 安全的多重簽名 (100 分)

**學習目標**: 理解 Rogue Key Attack（惡意密鑰攻擊）及 BIP 327 MuSig2 協議如何防禦

#### 技術細節

這個挑戰分為 5 個階段，從攻擊演示到完整的 MuSig2 協議實現。

#### Phase 1: Rogue Key Attack 演示

**漏洞場景 - 天真的密鑰聚合**:
```
P_agg = P_alice + P_bob  // 脆弱！
```

**攻擊方法**:
惡意的 Bob 可以選擇他的公鑰為：
```
P_bob_malicious = P_bob_real - P_alice
```

**攻擊結果**:
```
P_agg = P_alice + (P_bob_real - P_alice) = P_bob_real
```
Bob 單獨控制了「多重簽名」地址！這是嚴重的安全漏洞。

#### Phase 2: MuSig2 密鑰聚合（防禦）

**BIP 327 安全聚合公式**:

步驟 1 - 計算密鑰列表哈希:
```
L = H(P₁ || P₂ || ... || Pₙ)
```

步驟 2 - 計算每個參與者的係數:
```
aᵢ = H(L || Pᵢ)
```

步驟 3 - 加權聚合:
```
P_agg = a₁·P₁ + a₂·P₂ + ... + aₙ·Pₙ
```

**防禦原理**: 攻擊者無法在知道係數 `aᵢ` 之前選擇惡意公鑰，因為 `aᵢ` 依賴於所有公鑰的哈希。

#### Phase 3: Nonce 承諾輪（Commitment Round）

**MuSig2 雙輪 Nonce 協議**:

每個簽名者生成兩個隨機 nonce:
```
rᵢ,₁, rᵢ,₂ ← random()
Rᵢ,₁ = rᵢ,₁·G
Rᵢ,₂ = rᵢ,₂·G
```

承諾階段（防止並發簽名攻擊）:
```
commitᵢ = H(Rᵢ,₁ || Rᵢ,₂)
```

揭示階段:
```
廣播 Rᵢ,₁, Rᵢ,₂
驗證 H(Rᵢ,₁ || Rᵢ,₂) == commitᵢ
```

聚合 nonce:
```
R_agg = R₁,₁ + R₁,₂ + R₂,₁ + R₂,₂ + ...
```

#### Phase 4: 部分簽名生成

計算挑戰哈希:
```
e = H(R_agg || P_agg || m)
```

每個參與者創建部分簽名:
```
sᵢ = (rᵢ,₁ + rᵢ,₂) + e · aᵢ · dᵢ (mod n)
```
其中:
- `rᵢ,₁, rᵢ,₂` = 參與者的私有 nonce
- `e` = 挑戰哈希
- `aᵢ` = 密鑰聚合係數
- `dᵢ` = 參與者的私鑰

#### Phase 5: 簽名聚合

聚合所有部分簽名:
```
s = s₁ + s₂ + ... + sₙ (mod n)
```

最終 Schnorr 簽名:
```
sig = (R_agg, s)
```

這個簽名在鏈上與單個 Schnorr 簽名**完全相同**！

#### 實現函數

**`musigDemoAttack()`**
- 演示 Rogue Key 攻擊
- 生成 Alice 和 Bob 的誠實密鑰
- Bob 構造惡意公鑰 `P_bob_malicious = P_bob_honest - P_alice`
- 驗證聚合密鑰等於 Bob 的誠實公鑰

**`musigGenerateKeys(party)`**
- 為 MuSig2 協議生成密鑰對
- 支持 Alice 和 Bob 兩方

**`musig2AggregateKeys()`**
- 實現 BIP 327 密鑰聚合
- 計算 L = H(P₁ || P₂)
- 計算係數 a₁ = H(L || P₁), a₂ = H(L || P₂)
- 計算聚合公鑰 P_agg = a₁·P₁ + a₂·P₂

**`musig2GenerateNonces()`**
- 每個參與者生成兩個隨機 nonce
- 計算 nonce 點 R₁, R₂
- 生成承諾 H(R₁ || R₂)

**`musig2RevealNonces()`**
- 揭示 nonce 點
- 聚合所有 nonce: R_agg = ∑Rᵢ,ⱼ
- 進入簽名生成階段

**`musig2CreatePartialSigs()`**
- 計算挑戰 e = H(R_agg || P_agg || m)
- 每個參與者創建部分簽名 sᵢ = rᵢ + e·aᵢ·dᵢ

**`musig2AggregateSignature()`**
- 聚合部分簽名 s = s₁ + s₂
- 輸出最終簽名 (R_agg, s)
- 完成挑戰

#### MuSig2 安全性保證

1. **防禦 Rogue Key 攻擊**: 密鑰聚合係數使得攻擊者無法操縱聚合密鑰
2. **防禦並發簽名攻擊**: 承諾-揭示協議防止 Wagner 攻擊
3. **防禦相關 nonce 攻擊**: 雙 nonce 設計提供額外安全層
4. **完美隱私性**: 2-of-2 多重簽名在鏈上與單簽名無法區分
5. **手續費優化**: 約 20% 的交易空間節省
6. **可擴展性**: 可以擴展到 n-of-n multisig

#### 與 BIP 327 對應關係

| 遊戲階段 | BIP 327 協議階段 | 安全特性 |
|---------|-----------------|---------|
| Phase 1 | - | 演示漏洞 |
| Phase 2 | KeyAgg | 防 Rogue Key |
| Phase 3 | NonceGen + NonceAgg | 防並發攻擊 |
| Phase 4 | Sign | 部分簽名 |
| Phase 5 | PartialSigAgg | 最終簽名 |

### 挑戰 3: Taproot 地址構建器 (100 分)

**學習目標**: 理解 Taproot 地址的完整構建過程

#### 五個步驟

**步驟 1: 生成內部密鑰**
```javascript
P = 隨機私鑰的公鑰點
```
這是密鑰路徑花費的密鑰（最高效、最私密的方式）。

**步驟 2: 定義腳本條件**

可選腳本類型:
- **Timelock**: `OP_CHECKSEQUENCEVERIFY 144`（~24 小時）
- **2-of-3 Multisig**: `OP_CHECKMULTISIG 2 3`
- **Hash Lock**: `OP_HASH160 <hash> OP_EQUAL`
- **Oracle Signature**: `OP_CHECKSIG <oracle_pubkey>`

**步驟 3: 構建 Merkle 樹**
```
對於 1 個腳本: merkle_root = hash(script)
對於 2 個腳本: merkle_root = hash(hash(script1) + hash(script2))
對於 3+ 個腳本: 構建平衡二叉樹
```

**步驟 4: 調整密鑰**
```
tweak = H(P || merkle_root)
Q = P + tweak·G
```

這個調整將腳本樹承諾到公鑰中。

**步驟 5: 編碼為 Bech32m 地址**
```
地址 = bc1p + encode_bech32m(Q)
```

生成最終的 Taproot 地址（bc1p...格式）。

#### 實現函數

**`taprootStep1()` - `taprootStep5()`**
- 每個函數處理一個構建步驟
- 驗證前置步驟完成
- 更新進度條和視覺反饋
- 顯示中間結果

**`updateTaprootProgress()`**
- 更新進度條寬度（0-100%）
- 更新步驟計數器（0/5 到 5/5）
- 更新百分比顯示

#### Taproot 優勢

1. **隱私**: 所有 Taproot 輸出在鏈上看起來相同
2. **效率**: 密鑰路徑花費最高效
3. **靈活性**: 可以有多個備用花費條件
4. **選擇性披露**: 只需揭示使用的腳本

### 挑戰 4: MAST 偵探 (50 分)

**學習目標**: 理解 Merkelized Alternative Script Trees (MAST) 的選擇性披露特性

#### 技術細節

**MAST 的核心概念**:
- Merkle 樹中的每個葉子是一個腳本的哈希
- 花費時只需揭示:
  1. 使用的腳本
  2. 該腳本的 Merkle 證明
  3. 控制塊（包含內部密鑰和版本）

**鏈上數據**:
```
witness[0]: 簽名
witness[1]: 揭示的腳本
witness[2]: 控制塊（包含 Merkle 證明）
```

#### 實現函數

**`generateMysteryTx()`**
- 隨機選擇一個腳本類型
- 生成模擬的見證數據
- 顯示交易的見證字段

**`checkScript(scriptType)`**
- 驗證玩家的猜測
- 提供正確/錯誤反饋
- 解鎖 Merkle 證明解釋

#### MAST 優勢

1. **隱私保護**: 未使用的腳本永遠保持隱藏
2. **空間效率**: 只需揭示一個腳本，不管樹中有多少
3. **可擴展性**: 可以有數百個替代條件
4. **驗證效率**: Merkle 證明大小為 O(log n)

### 挑戰 5: 安全性分析測驗 (50 分)

**學習目標**: 鞏固對 Schnorr 和 Taproot 的全面理解

#### 測驗問題

**問題 1: Schnorr 相對於 ECDSA 的主要優勢**
- ✓ 正確答案: 線性簽名聚合特性
- 解釋: Schnorr 的線性性質允許通過 MuSig 進行簽名聚合

**問題 2: Nonce 重用的後果**
- ✓ 正確答案: 可以從兩個簽名計算私鑰
- 解釋: 使用代數即可恢復私鑰，這是災難性的

**問題 3: Taproot 的主要隱私優勢**
- ✓ 正確答案: 單簽、多簽和腳本輸出看起來完全相同
- 解釋: 所有 Taproot 輸出在區塊鏈上無法區分

**問題 4: Taproot 腳本路徑花費揭示的信息**
- ✓ 正確答案: 僅揭示使用的腳本 + 該腳本的 Merkle 證明
- 解釋: 這是 MAST 的核心——選擇性披露

**問題 5: Taproot 輸出密鑰的計算**
- ✓ 正確答案: Q = P + H(P || merkle_root) · G
- 解釋: 調整將腳本樹承諾到密鑰中

#### 實現函數

**`submitQuiz()`**
- 驗證所有 5 個問題都已回答
- 檢查每個答案是否正確
- 顯示即時反饋和解釋
- 要求全部正確才能完成挑戰

**`showFinalScore()`**
- 顯示最終得分（最高 450 分）
- 顯示完成證書
- 提供下載證書選項

## 評分系統

### 分數分配

| 挑戰 | 分數 | 總計 |
|------|------|------|
| 挑戰 1: Nonce 攻擊 | 100 + 100 | 200 |
| 挑戰 2: 密鑰聚合 | 100 + 100 | 200 |
| 挑戰 3: Taproot 構建 | 100 | 100 |
| 挑戰 4: MAST 偵探 | 50 | 50 |
| 挑戰 5: 安全測驗 | 50 | 50 |
| **總計** | | **600** |

### 評分機制

**`completeChallenge(challengeNum, points)`**
- 檢查是否已完成（防止重複得分）
- 更新總分並觸發動畫
- 解鎖下一個挑戰的導航按鈕
- 解鎖頂部階段選擇器的下一個按鈕

**分數動畫**:
```javascript
scoreElement.style.transform = 'scale(1.3)';
scoreElement.style.color = '#f7931a'; // Bitcoin 橙
setTimeout(() => {
    scoreElement.style.transform = 'scale(1)';
    scoreElement.style.color = '#4ade80'; // 綠色
}, 300);
```

## UI/UX 設計

### 配色方案

遵循密碼朋克和 Bitcoin 美學:

```css
/* 主要顏色 */
--bitcoin-orange: #f7931a;      /* Bitcoin 標誌橙色 */
--dark-bg: #0a0a0a;             /* 深色背景 */
--darker-bg: #151515;           /* 更深背景 */
--border: #2a2a2a;              /* 邊框 */
--text-primary: #e0e0e0;        /* 主要文字 */
--text-secondary: #9ca3af;      /* 次要文字 */
--success: #22c55e;             /* 成功綠 */
--danger: #ef4444;              /* 危險紅 */
```

### 按鈕樣式

**主要按鈕** (`.btn-primary`):
- 背景: Bitcoin 橙色 (#f7931a)
- 懸停: 發光效果
- 禁用: 灰色且不可點擊

**次要按鈕** (`.btn-secondary`):
- 透明背景，灰色邊框
- 用於導航和次要操作

**危險按鈕** (`.btn-danger`):
- 紅色主題
- 用於攻擊模擬等操作

### 導航系統

**挑戰底部導航**:
```html
<div class="challenge-nav">
    <button class="btn-secondary" onclick="switchStage(N-1)">← Previous</button>
    <button class="btn-primary" onclick="switchStage(N+1)" id="next-to-N+1" disabled>Next Challenge →</button>
</div>
```

**頂部階段選擇器**:
- 5 個按鈕代表 5 個挑戰
- 當前激活的按鈕高亮顯示
- 未解鎖的按鈕禁用並灰顯

### 進度反饋

**進度條** (Taproot Builder):
```html
<div class="progress-bar">
    <div class="progress-fill" style="width: 60%"></div>
</div>
<div class="progress-label">
    <span>Step 3/5</span>
    <span>60%</span>
</div>
```

**結果框**:
- 成功: 綠色背景，綠色邊框
- 錯誤: 紅色背景，紅色邊框
- 信息: 藍色背景，藍色邊框

## 加密學實現

### 哈希函數

使用簡化的哈希函數用於演示:

```javascript
function simpleHash(input) {
    let hash = 0;
    for (let i = 0; i < input.length; i++) {
        const char = input.charCodeAt(i);
        hash = ((hash << 5) - hash) + char;
        hash = hash & hash;
    }
    return Math.abs(hash).toString(16).padStart(64, '0');
}
```

**注意**: 這不是加密安全的哈希，僅用於教育演示。

### 模運算

實現 secp256k1 曲線的模運算:

```javascript
const SECP256K1_N = BigInt('0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141');

function modAdd(a, b) {
    const sum = BigInt('0x' + a) + BigInt('0x' + b);
    return (sum % SECP256K1_N).toString(16).padStart(64, '0');
}

function modSub(a, b) {
    const diff = BigInt('0x' + a) - BigInt('0x' + b);
    return ((diff % SECP256K1_N + SECP256K1_N) % SECP256K1_N).toString(16).padStart(64, '0');
}
```

### 橢圓曲線操作

**點乘法** (模擬):
```javascript
function pointMultiply(scalar, basePoint = 'G') {
    return simpleHash(scalar + basePoint);
}
```

**點加法** (模擬):
```javascript
function pointAdd(point1, point2) {
    return simpleHash(point1 + point2);
}
```

**注意**: 真實實現應使用 secp256k1 庫進行實際的橢圓曲線運算。

## 遊戲狀態管理

### 全局狀態

```javascript
let currentStage = 0;                              // 當前階段 (0-4)
let score = 0;                                     // 總分
let completedChallenges = [false, false, false, false, false];  // 完成狀態
let gameData = {
    // 挑戰 1
    leakedData: null,
    recoveredKey: null,
    forgedSignature: null,

    // 挑戰 2
    aliceKey: null,
    bobKey: null,
    aggregatedKey: null,
    multiSigSignature: null,

    // 挑戰 3
    internalKey: null,
    scriptTree: [],
    merkleRoot: null,
    tweakedKey: null,
    taprootAddress: null,

    // 挑戰 4
    mysteryTx: null
};
```

### 階段切換

**`switchStage(stageNum)`**:
1. 隱藏所有階段
2. 顯示選定的階段
3. 更新頂部按鈕狀態
4. 滾動到頁面頂部

### 解鎖機制

挑戰完成後:
1. 標記為已完成（防止重複）
2. 解鎖導航中的 "Next Challenge" 按鈕
3. 解鎖頂部階段選擇器中的下一個按鈕
4. 更新分數顯示

## 教育價值

### BIP 340: Schnorr 簽名

玩家將學習:
- ✅ Schnorr 簽名的數學原理
- ✅ nonce 重用的災難性後果
- ✅ 線性聚合特性（MuSig）
- ✅ 與 ECDSA 的比較優勢

### BIP 341: Taproot

玩家將學習:
- ✅ Taproot 地址的構建過程
- ✅ 密鑰路徑 vs 腳本路徑花費
- ✅ Merkle 樹和 MAST 概念
- ✅ 密鑰調整（key tweaking）
- ✅ 隱私和效率優勢

### 安全性概念

- ✅ 加密學漏洞利用
- ✅ 私鑰安全性
- ✅ 隱私技術
- ✅ 鏈上分析

## 完成證書

### 證書內容

完成所有挑戰後，玩家可以下載文本證書:

```
═══════════════════════════════════════════════════════════
    TAPROOT EXPLORER - COMPLETION CERTIFICATE
═══════════════════════════════════════════════════════════

This certifies that the holder has successfully completed
the Taproot Explorer challenge series and demonstrated
understanding of:

✓ BIP 340: Schnorr Signatures
  - Nonce reuse vulnerabilities and exploitation
  - Key aggregation with MuSig protocol
  - Signature linearity properties

✓ BIP 341: Taproot Addresses
  - Internal key and script tree construction
  - Merkle tree building and root calculation
  - Key tweaking: Q = P + H(P||m)·G
  - Taproot address generation (bc1p...)

✓ MAST (Merkelized Alternative Script Trees)
  - Selective script disclosure
  - Merkle proof verification
  - Privacy through hiding unused conditions

✓ Security Analysis
  - Understanding cryptographic vulnerabilities
  - Privacy benefits of Schnorr and Taproot
  - Best practices for signature generation

═══════════════════════════════════════════════════════════
FINAL SCORE: [score] / 600 points
DATE: [date]
═══════════════════════════════════════════════════════════
```

### 下載功能

**`downloadCertificate()`**:
- 生成包含得分和日期的證書
- 創建文本文件 blob
- 觸發下載
- 文件名: `taproot-explorer-certificate-[timestamp].txt`

## 技術實現細節

### HTML 結構

```html
<div id="game-container">
    <header>
        <h1>₿ TAPROOT SECURITY LAB</h1>
        <p class="subtitle">Interactive Challenges: BIP 340 & BIP 341</p>
    </header>

    <!-- 任務狀態 -->
    <div id="mission-status">
        <div class="mission-info">...</div>
        <div id="score-display">Score: <span id="score">0</span> / 600 points</div>
    </div>

    <!-- 階段選擇器 -->
    <div id="stage-selector">
        <button class="stage-btn active" onclick="switchStage(0)">Challenge 1</button>
        <!-- 更多按鈕... -->
    </div>

    <!-- 遊戲區域 -->
    <div id="game-area">
        <div class="stage active" id="stage-0"><!-- 挑戰 1 --></div>
        <div class="stage" id="stage-1"><!-- 挑戰 2 --></div>
        <!-- 更多階段... -->
    </div>
</div>
```

### CSS 組織

```css
/* 基礎樣式 */
* { /* 重置 */ }
body { /* 深色背景 */ }

/* 佈局 */
#game-container { /* 容器 */ }
header { /* 標題 */ }
#game-area { /* 主要區域 */ }

/* 組件 */
.challenge-box { /* 挑戰容器 */ }
.stage { /* 階段容器 */ }
.btn-primary { /* 主要按鈕 */ }
.btn-secondary { /* 次要按鈕 */ }

/* 特殊組件 */
.attack-simulator { /* 攻擊模擬器 */ }
.quiz-question { /* 測驗問題 */ }
.result-box { /* 結果框 */ }

/* 導航 */
.challenge-nav { /* 挑戰導航 */ }
#stage-selector { /* 階段選擇器 */ }

/* 響應式 */
@media (max-width: 768px) { /* 移動端 */ }
```

### JavaScript 模塊

```javascript
// ==================== 遊戲狀態 ====================
let currentStage, score, completedChallenges, gameData

// ==================== 工具函數 ====================
simpleHash(), randomHex(), modAdd(), modSub(),
pointMultiply(), pointAdd()

// ==================== 階段導航 ====================
switchStage(), updateScore(), completeChallenge()

// ==================== 挑戰 1 ====================
generateLeakedData(), calculatePrivateKey(), forgeSignature()

// ==================== 挑戰 2 ====================
generatePartyKeys(), aggregateKeys(), createMultiSig()

// ==================== 挑戰 3 ====================
taprootStep1() - taprootStep5(), updateTaprootProgress()

// ==================== 挑戰 4 ====================
generateMysteryTx(), checkScript()

// ==================== 挑戰 5 ====================
submitQuiz(), showFinalScore(), downloadCertificate()

// ==================== 初始化 ====================
document.addEventListener('DOMContentLoaded', init)
```

## 性能優化

### 延遲加載

- 只在需要時生成隨機數據
- 使用 `setTimeout()` 創建視覺延遲效果
- 防止 UI 阻塞

### 內存管理

- 只保存當前需要的遊戲數據
- 完成挑戰後清理中間狀態
- 使用標誌而非複製大對象

### 動畫優化

- 使用 CSS transitions 而非 JavaScript 動畫
- `transform` 和 `opacity` 優於佈局屬性
- 限制動畫持續時間（< 1 秒）

## 未來增強

### 計劃功能

1. **真實加密庫集成**
   - 使用 secp256k1 進行真實橢圓曲線運算
   - 實現真實的 BIP 340/341 規範

2. **更多挑戰**
   - Tapscript 編程挑戰
   - Lightning Network 路由
   - Covenant 結構

3. **成就系統**
   - 速度完成成就
   - 完美分數成就
   - 隱藏挑戰

4. **多語言支持**
   - 英文、中文、日文界面
   - 本地化加密術語

5. **社交功能**
   - 排行榜
   - 分享證書
   - 挑戰好友

## 開發者指南

### 添加新挑戰

1. **HTML**: 在 `game-area` 中添加新的 `<div class="stage">`
2. **CSS**: 添加任何特殊樣式
3. **JavaScript**:
   - 添加挑戰函數
   - 更新 `gameData` 結構
   - 添加到評分系統

### 測試檢查表

- [ ] 所有按鈕可點擊且有反饋
- [ ] 分數正確累加
- [ ] 導航按鈕正確解鎖
- [ ] 表單驗證正常工作
- [ ] 錯誤處理優雅
- [ ] 移動端響應式正常
- [ ] 控制台無錯誤
- [ ] 所有文本拼寫正確

### 調試技巧

**快速解鎖所有挑戰**:
```javascript
// 在瀏覽器控制台中
completedChallenges = [true, true, true, true, true];
score = 600;
document.querySelectorAll('.stage-btn').forEach(btn => {
    btn.disabled = false;
    btn.classList.remove('locked');
});
```

**快速切換到特定階段**:
```javascript
switchStage(3);  // 跳到挑戰 4
```

## 引用資源

### 官方文檔

- [BIP 340: Schnorr Signatures for secp256k1](https://github.com/bitcoin/bips/blob/master/bip-0340.mediawiki)
- [BIP 341: Taproot: SegWit version 1 spending rules](https://github.com/bitcoin/bips/blob/master/bip-0341.mediawiki)
- [BIP 342: Validation of Taproot Scripts](https://github.com/bitcoin/bips/blob/master/bip-0342.mediawiki)

### 學習資源

- Bitcoin Optech: Schnorr 和 Taproot 工作坊
- Bitcoin Core 開發者文檔
- "Mastering Bitcoin" 第 3 版

### 工具和庫

- **bitcoinjs-lib**: JavaScript Bitcoin 庫
- **secp256k1**: 橢圓曲線加密庫
- **bech32**: Bech32 和 Bech32m 編碼

## 許可證

本項目是 Cypherpunk Legacy 的一部分，遵循相同的開源許可證。

---

**最後更新**: 2025-01-13
**版本**: 1.0.0
**作者**: Cypherpunk Legacy Team
