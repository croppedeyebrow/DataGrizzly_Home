# WF09 공식 근거와 구현 확인 항목

확인일: 2026-10-24. 아래는 라이브러리 기능의 근거입니다. 작업 상태·API·자원 수치는 DataGrizzly의 설계 결정입니다.

- [LangGraph 개요](https://docs.langchain.com/oss/python/langgraph/overview): stateful workflow, 결정적 함수와 graph 조합, LangChain 없이도 사용 가능.
- [Persistence](https://docs.langchain.com/oss/python/langgraph/persistence): checkpointer와 store 구분, 메모리 저장은 프로세스 종료 후 복구 불가.
- [Interrupts](https://docs.langchain.com/oss/python/langgraph/interrupts): thread_id, 영속 checkpointer, interrupt/Command 재개, 재실행 시 부수 효과 주의.
- [Retrievers](https://docs.langchain.com/oss/python/integrations/retrievers): query를 Document 목록으로 변환하며 벡터DB에 한정되지 않음.
- [Document loaders](https://docs.langchain.com/oss/python/integrations/document_loaders/): 파일 형식별 어댑터 선택.
- [uv 프로젝트](https://docs.astral.sh/uv/guides/projects/): 프로젝트·가상환경·lock·실행. S20에서 현재 버전 지원 범위 확인.

설치 시 Python·langgraph·langchain-core·langgraph-checkpoint-postgres·psycopg의 상호 호환과 라이선스를 확인하고 uv.lock에 고정합니다. 문서 예제의 클래스·import를 추정으로 복사하지 말고 선택한 버전에서 import와 작은 graph 실행을 검증합니다. LangSmith 계정·유료 관리형 배포는 필수가 아닙니다. 로깅/추적 외부 전송은 기본 비활성입니다.
