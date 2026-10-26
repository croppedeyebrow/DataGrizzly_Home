# 05_data_model

```mermaid
erDiagram
 Project ||--o{ ProjectLog : groups
 Entry ||--o| ProjectLog : extends
 Entry ||--o{ Revision : versions
 Entry }o--o{ Taxonomy : tagged
 Entry }o--o{ Series : ordered
 Revision }o--o{ Media : references
 PublicationJob }o--o| SiteRelease : produces
 SiteRelease }o--o{ Revision : includes
 Revision ||--o{ SearchChunk : chunks
```
