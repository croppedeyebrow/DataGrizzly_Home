# 디스크 관리와 제외 기능

C: 여유23.4GiB에서 모든 도구를 설치하지 않습니다. Windows 저장소 설정에서 사용량을 조사하고 본인이 삭제 가능한 자료를 구분합니다. DB volume·미디어·Docker VHDX를 임의 삭제하거나 전체 prune을 실행하지 않습니다. 현재 다른 앱의 컨테이너 보유 여부도 확인해야 합니다.

한 번 설치→C: 여유 재측정→증가량 기록 순으로 진행합니다. 재생성 가능한 cache도 무조건 매번 지우지 않습니다. Windows 작업본과 Ubuntu Rust 작업본은 소스만 공유하고 node_modules·.gradle·target을 중복하지 않습니다. WSL의 가상 여유와 Windows 물리 여유는 다릅니다.

디자인 PNG64개는 참고용입니다. 전부 웹 번들에 포함하지 않습니다. Obsidian에는 문서·이미지만 넣고 node_modules·빌드 결과·DB·모델·비밀값은 넣지 않습니다.

## LLM과 Python 결정

이번 범위는 LLM 제외입니다. 모델 다운로드·llama.cpp·GPU runtime·추론 서버·질문 UI/API·벡터DB·embedding/reranker 서비스를 설치하거나 구현하지 않습니다. Python은 모델 실행 대신 LangChain·LangGraph 자료 처리 워커에 한정합니다. 언어만 바꾼다고 모델 가중치·context·추론 연산 비용이 자동 감소하지 않기 때문입니다.

향후 생성 기능을 다시 요청하면 별도 ADR로 모델 크기/양자화/메모리/한국어 품질/실제 지연/비용을 평가합니다. C++/Python 중 무엇을 쓰든 해당 평가가 먼저입니다. 현재 공개 검색은 모델 없이 동작합니다.

## Go 결정

기본 운영은 GitHub Actions·Docker Compose·작은 PowerShell/Bash/TypeScript 스크립트입니다. 기존 도구로 불편한 검사가 반복되며 단일 실행파일의 가치가 확인될 때만 Go CLI를 추가합니다. Go 서비스나 별도 Ops 플랫폼은 만들지 않습니다.

## 운영 분리

노트북은 개발 기기이며24시간 공개 서버가 아닙니다. 원격 단일 서버·CDN·백업 원칙은 유지합니다. 비용과 서버 용량은 M6에서 측정·결정하고 로컬 사양과 같다고 가정하지 않습니다.
