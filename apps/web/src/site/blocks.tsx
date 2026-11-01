/**
 * 개념: 글 블록을 종류별로 화면에 놓는다.
 * 목적: 코드와 수식은 텍스트로만 보여주고, 모르는 블록은 빈 칸으로 지우지 않는다.
 * 사용처: 노트 상세와 리서치 상세.
 */
import type { ViewBlock } from '../scripts/research-types'

const markUrl = '/media/example-mark.svg'
const panel = 'mb-4 border border-rule bg-paper px-4 py-3'

export function BlockList({ blocks }: { blocks: ViewBlock[] }) {
  return (
    <>
      {blocks.map((block, index) => (
        <BlockView key={`${block.kind}-${index}`} block={block} />
      ))}
    </>
  )
}

function BlockView({ block }: { block: ViewBlock }) {
  if (block.kind === 'paragraph') {
    return (
      <p className="mb-4">
        {(block.parts ?? []).map((part, index) =>
          part.kind === 'link' ? (
            <a key={index} className="text-mark" href={part.href} rel="noreferrer">
              {part.href}
            </a>
          ) : (
            <span key={index}>{part.text}</span>
          ),
        )}
      </p>
    )
  }
  if (block.kind === 'heading') {
    return block.level === 3 ? (
      <h3 id={block.anchor} className="mb-3 mt-6 text-[1.15rem] leading-snug">
        {block.text}
      </h3>
    ) : (
      <h2 id={block.anchor} className="mb-3 mt-10 text-[1.35rem] leading-snug">
        {block.text}
      </h2>
    )
  }
  if (block.kind === 'list') {
    const items = (block.items ?? []).map((item) => (
      <li key={item} className="mb-2">
        {item}
      </li>
    ))
    return block.ordered ? <ol className="mb-4 list-decimal pl-6">{items}</ol> : <ul className="mb-4 list-disc pl-6">{items}</ul>
  }
  if (block.kind === 'quote') {
    return (
      <blockquote className="mb-4">
        <p className="mb-4">{block.text}</p>
        {block.citation ? <p className="mb-4 text-quiet">{block.citation}</p> : null}
      </blockquote>
    )
  }
  if (block.kind === 'table') {
    return (
      <div className="table-scroll mb-4">
        <table>
          {block.caption ? <caption>{block.caption}</caption> : null}
          <thead>
            <tr>
              {(block.header ?? []).map((cell) => (
                <th key={cell}>{cell}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {(block.rows ?? []).map((row, rowIndex) => (
              <tr key={rowIndex}>
                {row.map((cell, cellIndex) => (
                  <td key={cellIndex}>{cell}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    )
  }
  if (block.kind === 'image') {
    return block.imageName === 'example-mark.svg' && !block.missing ? (
      <img src={markUrl} alt={block.alt ?? ''} width="64" height="64" />
    ) : (
      <p className={panel}>{block.alt} 파일을 찾지 못했다.</p>
    )
  }
  if (block.kind === 'code') {
    return (
      <>
        <p className="mb-4 text-quiet">언어 {block.language}. 파일명이 아직 없다.</p>
        <pre className="code-block mb-4 border border-rule bg-paper p-3">
          <code>{block.text}</code>
        </pre>
      </>
    )
  }
  if (block.kind === 'math' && block.state === 'shown') {
    return (
      <>
        <p className="mb-4 text-quiet">수식은 실행하지 않는다.</p>
        <pre className="math-block mb-4 border border-rule bg-paper p-3">
          <code>{block.tex}</code>
        </pre>
      </>
    )
  }
  if (block.kind === 'math') {
    return <p className={panel}>{block.message}</p>
  }
  if (block.kind === 'callout') {
    return <p className={block.tone === 'warning' ? panel : panel}>{block.text}</p>
  }
  if (block.kind === 'related') {
    return block.href ? (
      <p className="mb-4">
        <a className="text-mark" href={block.href}>
          {block.title}
        </a>
      </p>
    ) : (
      <p className={panel}>관련 글을 공개 목록에서 찾지 못했다.</p>
    )
  }
  return <p className={panel}>{block.message || '지원하지 않는 블록이다.'}</p>
}
