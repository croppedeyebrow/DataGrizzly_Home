/**
 * 개념: 자동저장 상태.
 * 목적: 늦은 응답이 최신 입력을 덮지 않고, 409 뒤에는 자동으로 다시 저장하지 않는다.
 * 사용처: 편집 화면과 autosave.test.ts.
 */

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'failed' | 'conflict' | 'invalid'

export type SaveState = {
  status: SaveStatus
  generation: number
  sentGeneration: number
  expectedVersion: number
  conflictVersion: number | null
}

export type SaveEvent =
  | { type: 'load'; version: number }
  | { type: 'edit' }
  | { type: 'send' }
  | { type: 'saved'; generation: number; version: number }
  | { type: 'conflict'; version: number }
  | { type: 'failed' }
  | { type: 'invalid' }
  | { type: 'acceptServerVersion' }

/**
 * 개념: 저장 상태 전이.
 * 목적: 보낸 세대와 현재 세대가 다르면 화면 문구를 완료로 바꾸지 않는다.
 * 사용처: 편집 화면의 저장 결과 처리.
 */
export function reduceSave(state: SaveState, event: SaveEvent): SaveState {
  if (event.type === 'load') {
    return {
      status: 'saved',
      generation: 0,
      sentGeneration: 0,
      expectedVersion: event.version,
      conflictVersion: null,
    }
  }
  if (event.type === 'edit') {
    return {
      ...state,
      status: state.status === 'conflict' || state.status === 'saving' ? state.status : 'idle',
      generation: state.generation + 1,
    }
  }
  if (event.type === 'send') {
    if (state.status === 'conflict' || state.status === 'saving') return state
    return { ...state, status: 'saving', sentGeneration: state.generation }
  }
  if (event.type === 'saved') {
    if (event.generation !== state.generation) {
      return { ...state, expectedVersion: event.version, status: 'idle' }
    }
    return {
      ...state,
      expectedVersion: event.version,
      status: 'saved',
      sentGeneration: event.generation,
    }
  }
  if (event.type === 'conflict') {
    return { ...state, status: 'conflict', conflictVersion: event.version }
  }
  if (event.type === 'failed') return { ...state, status: 'failed' }
  if (event.type === 'invalid') return { ...state, status: 'invalid' }
  if (state.conflictVersion === null) return state
  return {
    ...state,
    expectedVersion: state.conflictVersion,
    status: 'idle',
    conflictVersion: null,
    sentGeneration: state.generation,
  }
}

/**
 * 개념: 자동으로 보낼 수 있는 상태.
 * 목적: 충돌 중에는 최신 버전 확인 없이 다시 보내지 않는다.
 * 사용처: 디바운스 타이머.
 */
export function canAutosave(state: SaveState): boolean {
  return state.status === 'idle' && state.generation !== state.sentGeneration
}
