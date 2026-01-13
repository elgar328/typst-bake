# typst-bake 크레이트 개발 계획

## 개요

**typst-bake**는 Typst 문서를 PDF로 렌더링하는 Rust 크레이트입니다. 모든 리소스(템플릿, 폰트, 패키지)를 빌드 타임에 바이너리에 "굽는(bake)" 방식으로, 완전한 오프라인 환경에서도 동작합니다.

## 핵심 특징

- **완전 오프라인**: 런타임에 파일시스템/인터넷 접근 불필요
- **올인원 바이너리**: 템플릿, 폰트, 패키지가 모두 임베드됨
- **간단한 API**: `document!()` 매크로 한 줄로 Document 생성
- **단일 설정**: Cargo.toml에 템플릿 경로만 지정

## 크레이트 구조

```
typst-bake (workspace)
├── typst-bake-macros/     # proc-macro 크레이트
│   ├── Cargo.toml
│   └── src/lib.rs         # document!() 매크로 구현
│
└── typst-bake/            # 메인 크레이트
    ├── Cargo.toml
    └── src/
        ├── lib.rs         # 공개 API + 매크로 re-export
        ├── document.rs    # Document 구조체
        └── resolver.rs    # 템플릿/패키지 리졸버
```

## 의존성

### typst-bake (메인 크레이트)
- `typst` - Typst 컴파일러
- `typst-pdf` - PDF 렌더링
- `typst-as-lib` - Typst 래퍼 (내부 사용)
- `include_dir` - 폴더 임베딩
- `derive_typst_intoval` - 데이터 변환
- `typst-bake-macros` - proc-macro (re-export)

### typst-bake-macros (proc-macro 크레이트)
- `proc-macro2`, `quote`, `syn` - proc-macro 기본
- `typst-syntax` - AST 파싱
- `ureq` - 패키지 다운로드
- `flate2`, `binstall-tar` - 압축 해제
- `toml` - 설정 파싱
- `walkdir` - 디렉토리 탐색

## API 설계

### 프로젝트 설정

**Cargo.toml**:
```toml
[package.metadata.typst-bake]
template-dir = "./templates"

[dependencies]
typst-bake = "0.1"
```

build.rs 불필요, build-dependencies 불필요!

### 기본 사용법

```rust
use derive_typst_intoval::{IntoValue, IntoDict};

#[derive(IntoValue, IntoDict)]
struct Inputs {
    title: String,
    items: Vec<Item>,
}

#[derive(IntoValue, IntoDict)]
struct Item {
    name: String,
    price: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let inputs = Inputs {
        title: "My Document".into(),
        items: vec![
            Item { name: "Widget".into(), price: 10.0 },
        ],
    };

    // document!() 매크로가 템플릿과 패키지를 임베드한 Document를 생성
    let pdf = typst_bake::document!("main.typ")
        .with_font(include_bytes!("fonts/myfont.ttf"))
        .with_inputs(inputs)
        .to_pdf()?;

    std::fs::write("output.pdf", pdf)?;
    Ok(())
}
```

### API 메서드

| 메서드 | 필수 | 설명 |
|--------|------|------|
| `document!(entry)` | ✅ | 템플릿/패키지 임베드 + Document 생성 |
| `.with_font(bytes)` | ⚪ | 폰트 추가 (체이닝 가능) |
| `.with_inputs(data)` | ⚪ | 데이터 전달 |
| `.to_pdf()` | ✅ | PDF 생성 |

### 데이터 전달 규칙

1. 최상위 구조체 이름은 관용적으로 `Inputs` 사용 (Typst의 `sys.inputs`와 일치)
2. `#[derive(IntoValue, IntoDict)]` 필요

```typst
// Typst 템플릿에서
#import sys: inputs

#inputs.title
#inputs.items
```

## 내부 구현

### document!() 매크로가 생성하는 코드

```rust
// typst_bake::document!("main.typ") 호출 시 생성되는 코드
{
    use ::typst_bake::__internal::{Document, include_dir, Dir};

    static TEMPLATES: Dir<'static> = include_dir!("/abs/path/to/templates");
    static PACKAGES: Dir<'static> = include_dir!("/path/to/cache/packages");

    Document::__new(&TEMPLATES, &PACKAGES, "main.typ")
}
```

매크로가 `Document`까지 직접 생성하므로, 라이브러리 코드가 사용자 크레이트의 모듈을 참조하는 문제가 없습니다.

### Document 구조체

```rust
impl Document {
    // 내부용 생성자 (매크로에서만 호출)
    #[doc(hidden)]
    pub fn __new(
        templates: &'static Dir<'static>,
        packages: &'static Dir<'static>,
        entry: &str,
    ) -> Self {
        Self {
            templates,
            packages,
            entry: entry.to_string(),
            fonts: Vec::new(),
            inputs: None,
        }
    }
}
```

## 컴파일 타임 동작

```
사용자 설정:
  Cargo.toml → template-dir = "./templates"
       ↓
컴파일 타임 (document!() 매크로 실행):
  1. CARGO_MANIFEST_DIR에서 사용자 프로젝트 경로 획득
  2. Cargo.toml 메타데이터에서 template-dir 읽기
  3. 템플릿 폴더의 .typ 파일 스캔 → 패키지 import 파싱
  4. 패키지 다운로드 → 시스템 캐시 디렉토리 (진행 상황 터미널 출력)
  5. 전이적 의존성 자동 해결
  6. include_dir! + Document 생성 코드 반환
       ↓
바이너리:
  - 템플릿과 패키지가 static 변수로 임베드됨
  - 런타임에 파일시스템/인터넷 접근 불필요
```

### 패키지 캐시 위치

proc-macro는 OUT_DIR에 접근할 수 없으므로, 시스템 캐시 디렉토리 사용:
- Linux: `~/.cache/typst-bake/packages/`
- macOS: `~/Library/Caches/typst-bake/packages/`
- Windows: `%LOCALAPPDATA%\typst-bake\packages\`

장점: 여러 프로젝트 간 패키지 캐시 공유 가능

### 캐시 관리

캐시를 무시하고 패키지를 다시 다운로드하려면:
```bash
TYPST_BAKE_REFRESH=1 cargo build
```

### 증분 컴파일

`include_dir!`이 템플릿 폴더의 파일 변경을 감지하므로, `.typ` 파일 수정 시 자동으로 재컴파일됩니다. 문제가 발생하면 모든 `.typ` 파일에 대한 더미 `include_bytes!`를 생성하여 의존성을 명시적으로 추가합니다.

## 파일 구조

```
typst-bake/
├── Cargo.toml                 # workspace
├── typst-bake-macros/
│   ├── Cargo.toml             # proc-macro = true
│   └── src/
│       ├── lib.rs             # document!() 매크로
│       ├── config.rs          # Cargo.toml 메타데이터 파싱
│       ├── scanner.rs         # .typ 파일 스캔, import 파싱
│       └── downloader.rs      # 패키지 다운로드
│
├── typst-bake/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs             # 공개 API + 매크로 re-export
│       ├── document.rs        # Document 구조체
│       └── resolver.rs        # 템플릿/패키지 리졸버
│
└── examples/
    └── basic/
        ├── Cargo.toml
        ├── src/main.rs
        ├── templates/
        │   └── main.typ
        └── fonts/
            └── font.ttf
```

## 구현 단계

### Phase 1: Workspace 설정
- [ ] workspace Cargo.toml 생성
- [ ] typst-bake-macros 크레이트 생성
- [ ] typst-bake 크레이트 생성

### Phase 2: proc-macro 구현 (typst-bake-macros)
- [ ] Cargo.toml 메타데이터 읽기
- [ ] typst-as-lib에서 패키지 다운로드 코드 이식
- [ ] 시스템 캐시 디렉토리 관리 (dirs 크레이트)
- [ ] 다운로드 진행 상황 터미널 출력
- [ ] document!() 매크로 구현

### Phase 3: 메인 크레이트 (typst-bake)
- [ ] 매크로 re-export
- [ ] Document 구조체 구현 (__new 내부 생성자)
- [ ] 템플릿/패키지 리졸버 구현 (typst-as-lib 활용)
- [ ] with_font(), with_inputs(), to_pdf() 메서드

### Phase 4: 테스트 및 예제
- [ ] 기본 예제 작성
- [ ] 패키지 사용 예제
- [ ] 이미지 포함 예제

### Phase 5: 문서화 및 배포
- [ ] README.md 작성
- [ ] API 문서 주석
- [ ] crates.io 배포

## 기존 코드 참조

typst-as-lib feature/auto-package-bundling 브랜치에서 이식할 코드:

- `extract_packages()` - 템플릿에서 패키지 추출
- `download_packages()` - 패키지 다운로드 및 전이적 의존성 해결
- `parse_packages_from_source()` - AST 파싱
- `EmbeddedPackageResolver` - 패키지 리졸버 (수정 필요)

## 주의사항

1. **폰트 정책**: 완전한 오프라인/재현성을 위해 임베드된 폰트만 사용 (시스템 폰트 무시)
2. **패키지 캐싱**: 시스템 캐시 디렉토리에 저장, 프로젝트 간 공유
3. **첫 컴파일**: 패키지 다운로드로 인해 시간이 걸릴 수 있음 (진행 상황 터미널 출력)
4. **다운로드 중단**: 네트워크 문제 시 사용자가 Ctrl+C로 중단 가능
5. **바이너리 크기**: 폰트/패키지가 많으면 바이너리 커짐

## 향후 확장 가능성

- HTML 출력 지원 (typst-html)
- SVG 출력 지원 (typst-svg)
- 템플릿 핫 리로드 (개발 모드)
- 커스텀 패키지 소스 지원
- 바이너리 압축 (include_dir의 flate2 feature)
