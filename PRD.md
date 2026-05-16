# Product Requirement Document (PRD) — Project RUTA

## 1. General Information

* **Product / Feature Name:** Roota
* **Version:** v1.0
* **Status:** Draft / Ready for Review

---

## 2. Executive Summary

### Description

> **Roota** is an **Edge AI** (On-Device Artificial Intelligence) solution dedicated to bridging the digital divide for older adults by automating operating system tasks. Utilizing a compact, local language model, the application translates simple natural language commands (such as drafting an email or opening a file) into immediate actions on the computer. This completely eliminates complex interfaces and confusing navigation menus, delivering a 100% inclusive user experience.

### Main Objective

To facilitate and democratize personal computer usage for senior citizens through an intelligent automation assistant that operates autonomously, fluidly, and **completely disconnected from the internet**.

### User Value Proposition

* **Zero Technical Complexity:** The user simply states what they need in plain language, and the AI handles execution or step-by-step guidance in real-time.
* **Absolute Privacy & Security:** Because processing occurs entirely on the edge (locally), sensitive documents, personal emails, and digital footprints never leave the device. This provides absolute protection against data leaks and online exploits targeting the elderly.

---

## 3. Problem & Market Opportunity

### Current Problem

Older adults frequently feel lost, intimidated, or anxious when interacting with modern computer interfaces. They require constant supervision or a guided framework to understand what they are doing and why.

### Market Opportunity

Modern technology design cycles move faster than generational adaptation. Simple everyday workflows—such as opening a web browser or navigating a nested file directory—present high cognitive loads for seniors. RUTA captures this unserved demographic by abstracting the OS layer into a conversational and visual instruction map.

---

## 4. Target User Profile

### User Persona: The Digitally Vulnerable Senior

* **Age Range:** 55–75 years old.
* **Context:** Uses mobile devices or basic desktop applications for essential communication (messaging, video calls, looking up information, accessing online banking, or checking in with family).
* **Behavioral Traits:** Despite regular exposure to digital screens, they feel deeply insecure dealing with software updates, UI redesigns, or counter-intuitive setup flows. They lean heavily on family members to execute trivial tasks out of fear of breaking something, losing data, or falling victim to cyber fraud. They long for digital independence but find current tools tailored exclusively to advanced or younger users.

### User Needs vs. System Frustrations

| User Needs | Current System Frustrations |
| --- | --- |
| Clear, step-by-step, paced visual instructions. | Fear of triggering irreversible errors or deleting files. |
| Non-technical, direct language without jargon. | Cognitive confusion from cluttered interfaces and regular UI updates. |
| Immediate explicit confirmation of successful actions. | Constant codependency on relatives for simple tasks. |
| A calming presence that mimics human companionship. | Deep-seated distrust of hidden background automation or AI. |
| Automatic threat detection (scams, suspicious links). | Feeling completely excluded from modern technological growth. |

---

## 5. Use Cases & User Flow

### Core Use Case Framework

* **As a:** Senior User (55–75 years old)
* **I want to:** Give a command in simple, natural language (spoken or typed).
* **So that:** The system can guide me visually or execute the task automatically without me needing to navigate complex application menus.

### Primary Happy Path User Flow

```
[1. Application Launch] 
   └── User opens RUTA -> High-contrast UI with large typography.
   └── Screen displays a single central input box: "¿Qué tarea quieres que haga por ti hoy?" ("What task can I do for you today?")

[2. Input Command]
   └── User types/speaks: "Escribir un correo para mi hija Elena" ("Write an email to my daughter Elena").

[3. Safety Confirmation Gate]
   └── High-visibility modal appears with massive buttons: [YES (Green)] / [NO (Red)].
   └── System confirms: "Voy a empezar a escribir un correo para Elena. ¿Está bien?" ("I'm going to start writing an email to Elena. Is that okay?").

[4. Guided Execution]
   └── User clicks "YES" -> System interacts with OS or highlights target areas, eliminating manual folder/application searching.

```

---

## 6. Product Requirements

### Non-Functional Requirements

#### Performance & Latency

* The system must respond in near real-time (minimal latency) to avoid user confusion during live step-by-step guidance.
* Screen element detection and spatial visual overlays must render smoothly ($>30\text{ FPS}$) even on mid-range or legacy consumer hardware.

#### Accessibility Specifications

* **Visual Architecture:** High contrast ratios (WCAG AAA compliant), ultra-large typography, and explicit visual anchors.
* **AI Clarity:** The model output must be concise, direct, and conversational, highlighting exactly *where* and *what* actions to take on screen.

### Prioritization Matrix

* **Must Have:** Local LLM baseline processing, high-contrast accessible UI window, transparent spatial overlay framework, basic Windows Explorer navigation guidance.
* **Should Have:** Local Speech-to-Text (STT) and Text-to-Speech (TTS) pipelines, basic runtime application state error detection.
* **Could Have:** Deep browser integration (Chrome/Edge navigation), native platform automated notification filters.
* **Won’t Have (v1.0):** Cloud analytics synchronization, multi-tenant profile swapping.

---

## 7. UX / UI Specification

### Core Design Principles

* **Cognitive Load Reduction:** Present exactly **one** distinct action at a time.
* **Progressive Disclosure:** Surface configuration paths only when functionally imperative.
* **Accessibility-First:** Tailored explicitly around senior tactile limitations and vision changes.
* **Calm UI:** Minimalist, quiet layouts with predictable interaction points.
* **Spatial Guidance:** Heavy reliance on context-aware arrows, highlights, and screen overlays.
* **Recognition Over Recall:** Guide the user visually rather than forcing them to memorize sequences.
* **Positive Reinforcement:** Consistent, reassuring feedback upon step completions.
* **Error-Tolerant UX:** Forgiving error states with single-click recovery channels.
* **Human-Centered AI:** Designed to act like a patient, friendly companion rather than an analytical utility tool.

### Design Benchmarks & References

* Apple Setup Assistant (Clean onboarding layout)
* Microsoft Fluent Design (System-level accessibility frameworks)
* Duolingo UX (Gamified, digestible progress and reinforcement mechanics)
* Notion AI & Google Assistant (Contextual inline command patterns)

### Visual Mockups (Wireframe Concepts)

#### Floating Assistant UI

```txt
┌──────────────────────────────────────┐
│  😊 How can I help you today?       │
└──────────────────────────────────────┘

```

#### Guided UI Overlay

```txt
Step 1:
Left-click on the "Downloads" folder below.

              ⭕ [Pulsing Visual Anchor]

```

#### Positive Feedback State

```txt
✅ Perfect!
Now let's proceed to the next step.

```

---

## 8. Technical Architecture

### Component Technology Stack

| Layer | Selected Technology | Purpose |
| --- | --- | --- |
| **Desktop Application Framework** | Tauri | Lightweight native desktop shell container |
| **System Backend** | Rust | Secure, low-footprint OS runtime integration |
| **Rapid Prototyping Layer** | Python | Fast pipeline assembly and testing |
| **Accessibility API Engine** | Windows UI Automation API | Target UI node mapping, reading, and traversal |
| **Visual Presentation Window** | Transparent Always-On-Top Overlay | Native drawing plane for markers and arrows |
| **Local AI Runtime Engine** | Ollama / llama.cpp | On-device, resource-optimized LLM inference |
| **Target LLM Weights** | Qwen2.5 3B | High-performance, low-parameter local language model |
| **Speech-to-Text Layer** | whisper.cpp | Offline, ultra-fast local voice transcription |
| **Text-to-Speech Layer** | Piper TTS / Native Windows TTS | Low-latency local voice generation |
| **Prototype Interface Shell** | PySide6 / CustomTkinter | Fast internal testing interface |

### Modular Architecture Breakdown

```
                  ┌──────────────────────────────┐
                  │      User Intent (Voice/UI)  │
                  └──────────────┬───────────────┘
                                 │
                                 ▼
                    ┌──────────────────────────┐
                    │   Intent Recognition     │
                    └────────────┬─────────────┘
                                 │
                                 ▼
┌──────────────────────┐    ┌──────────────────────────┐    ┌──────────────────────┐
│  Accessibility Engine│ ──►│   Local LLM Engine       │ ──►│ Context/State Manager│
│ (Reads Window Nodes) │    │ (Generates Instructions) │    │ (Tracks Goals/Steps) │
└──────────────────────┘    └────────────┬─────────────┘    └──────────────────────┘
                                         │
                                         ▼
                    ┌──────────────────────────┐    ┌──────────────────────┐
                    │     Decision Engine      │ ──►│    Overlay Engine    │
                    │  (Evaluates Next Action) │    │ (Draws Visual Anchors│
                    └──────────────────────────┘    └──────────────────────┘

```

#### 1. Accessibility Engine

Scans and parses native desktop active layout attributes.

* **Reads:** Screen windows, active buttons, input nodes, drop-down menus, spatial UI coordinates, focus states.
* **Underlying Utilities:** UI Automation API, `pywinauto`, `FlaUI`.
* **Sample Engine Output Structure:**

```json
{
  "window": "Explorer",
  "elements": [
    {
      "type": "button",
      "text": "Downloads",
      "x": 120,
      "y": 340
    }
  ]
}

```

#### 2. Intent Recognition

Determines the functional objective hidden behind natural conversation.

* **Input Phrase:** *"I want to move this photo over here"*
* **Output Node:**

```json
{
  "intent": "move_file",
  "target": "photo.jpg"
}

```

#### 3. Local LLM Engine

Generates clear contextual instruction steps using an empathetic linguistic persona.

* **System Prompt Constraint:**

```text
Role: You are a patient, clear technical companion for older adults.
Rules: Explain exactly ONE step at a time. Do NOT use technical jargon. 
Never execute actions automatically without explicit safety verification.

```

#### 4. Context / State Manager

Maintains the session timeline memory and task history parameters.

* **Live Context State Snapshot:**

```json
{
  "goal": "move_file",
  "step": 2,
  "completed": false
}

```

#### 5. Decision Engine

Combines the UI layout structure with LLM context paths to select the next optimal screen action.

```
User requests a file modification
     ↓
System verifies active File Explorer instance
     ↓
System detects selected target file coordinates
     ↓
LLM constructs user-friendly single-step directive
     ↓
Overlay module renders an explicit target guide on screen

```

#### 6. Overlay Engine

Draws visual indicators directly onto a dedicated transparent screen plane.

* **Render Assets:** Target pointers (👉), pulsing boundary circles (⭕), localized focus highlights (🟦).
* **Execution Strategy:** Borderless system-level overlay window locked to `always-on-top`.

#### 7. Audio Translation Layer (Voice Layer)

* **STT (Speech-to-Text):** Processes offline vocal prompts into clean system string buffers using *Whisper.cpp*.
* **TTS (Text-to-Speech):** Synthesizes structural feedback responses into audio speech streams using *Piper TTS* or native platform audio pipelines.

#### 8. State Detection Engine

Continuously validates if the user has successfully completed the requested instruction.

* **State Shift Delta Validation:**

```text
Initial State: Context context-menu index value is NULL
Action Logged: User triggers physical right-click input on target coordinates
Target State: System detects visible context menu tree structure
Result: Step 2 flagged as SUCCESSFULLY COMPLETED.

```

#### 9. Safety Layer

> ⚠️ **CRITICAL RUNTIME RULE:** The application must never directly control input hardware registers. The core system **does NOT** move the mouse cursor, **does NOT** fake keyboard keystrokes, and **does NOT** automate background file operations directly. It acts strictly as a persistent, safe visual guide.

---

## 9. Repository Directory Layout

```text
app/
├── accessibility/   # UI Accessibility Tree scanners and coordinate parsers
├── overlay/         # Transparent rendering surface components
├── llm/             # On-device LLM management interfaces and quantization tools
├── voice/           # Local voice processing engines (STT/TTS)
├── orchestration/   # Core routing engine syncing LLM data with OS snapshots
├── prompts/         # Versioned prompt text templates
├── state/           # User execution tracking state machines
├── ui/              # User-facing application configuration layout windows
├── telemetry/       # Localized debugging and error-capturing systems
└── config/          # Local platform and model parameter settings

```

---

## 10. Recommended Development Roadmap

### Phase 1: Minimal Viable Product (MVP)

* **Scope:** Limited strictly to Windows Explorer operations (e.g., finding and moving files).
* **UI/UX:** Transparent visual helper layout containing simple text directions.
* **Technical Stack:** Python, `pywinauto`, Local Ollama runtime instance.

### Phase 2: Conversational Multi-Modal Expansion

* **Scope:** Integration of local audio interfaces (STT/TTS models).
* **UI/UX:** Enhanced contextual fallback detection and user onboarding flows.
* **Technical Stack:** Native compilation optimization incorporating *Whisper.cpp* and *Piper TTS*.

### Phase 3: Application Ecosystem Scaling

* **Scope:** Introduce instruction handling templates for primary standalone tools (Google Chrome navigation, Gmail workflows, basic Microsoft Office tasks).
* **UI/UX:** Advanced multi-window overlay anchoring systems.

### Phase 4: Production Refactoring

* **Scope:** Complete optimization overhaul to guarantee minimal system resource footprint.
* **Technical Stack:** Deprecate Python prototyping layers in favor of a full production native rewrite using **Rust** and **Tauri**.