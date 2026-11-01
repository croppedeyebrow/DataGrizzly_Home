# 20 Python 문서 처리 아키텍처

Python의 제품 역할은 LangChain 어댑터와 LangGraph 상태 처리로 제한합니다. 모델 호출·일반 CMS API·직접 공개 발행은 제외합니다.

설계 본문: [[WF02_ARCHITECTURE]], [[WF03_STATE_MACHINE]], [[WF04_DATA_API]], [[WF06_OPERATIONS]].

일반 방문자는 React 정적 페이지와, 검색이 필요할 때 Rust API·PostgreSQL을 사용합니다. 관리자 자료 등록은 Rust API에 durable job으로 저장됩니다. Python 워커는 작업을 claim하여 LangChain loader/Retriever와 Rust 라이브러리를 연결하고 checkpointer에 상태를 저장합니다. 승인도 Rust API에 command로 저장되므로 워커 부재와 분리됩니다. 워커 재개 후 최종 초안 저장은 Rust API가 수행합니다.
