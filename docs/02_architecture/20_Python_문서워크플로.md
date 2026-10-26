# 20 Python 문서 처리 아키텍처

Python의 제품 역할은 LangChain 어댑터와 LangGraph 상태 처리로 제한합니다. 모델 호출·일반 CMS API·직접 공개 발행은 제외합니다.

설계 본문: [[WF02_ARCHITECTURE]], [[WF03_STATE_MACHINE]], [[WF04_DATA_API]], [[WF06_OPERATIONS]].

일반 방문자는 기존 TypeScript→Kotlin→PostgreSQL 흐름을 유지합니다. 관리자 자료 등록은 Kotlin에 durable job으로 저장됩니다. Python 워커는 작업을 claim하여 LangChain loader/Retriever와 Rust CLI를 연결하고 checkpointer에 상태를 저장합니다. 승인도 Kotlin에 command로 저장되므로 워커 부재와 분리됩니다. 워커 재개 후 최종 초안 저장은 Kotlin이 수행합니다.
