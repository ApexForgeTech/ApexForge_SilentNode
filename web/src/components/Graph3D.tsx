import { useRef, useEffect, useCallback, useMemo, useState } from 'react'
import ForceGraph3D from 'react-force-graph-3d'
import {
  AdditiveBlending,
  CanvasTexture,
  Color,
  Sprite,
  SpriteMaterial,
} from 'three'
import type { SNode, SEdge, SoulData, TrailEvent } from '../types'
import { NODE_COLORS } from '../types'

interface GNode {
  id: string
  node_type: string
  custom_color?: string | null
  content: string
  gravity: number
  entropy: number
  velocity: number
  is_ghost: boolean
  is_fossil: boolean
  is_void: boolean
  entropy_state?: string
  velocity_state?: string
  visual_weight?: number
  contagion_heat?: number
  soul_color?: string
  x?: number; y?: number; z?: number
}

interface GLink {
  source: string
  target: string
  edge_type: string
  weight: number
}

interface Props {
  nodes: SNode[]
  edges: SEdge[]
  selectedId: string | null
  onSelect: (node: SNode | null) => void
  dim?: boolean
  trail?: TrailEvent[]
  souls?: SoulData[]
}

// Cache textures so we don't recreate canvases on every render
const textureCache = new Map<string, CanvasTexture>()

function makeGlowTexture(hex: string, alpha: number, selected: boolean): CanvasTexture {
  const key = `${hex}_${alpha.toFixed(2)}_${selected}`
  if (textureCache.has(key)) return textureCache.get(key)!

  const size = 256
  const canvas = document.createElement('canvas')
  canvas.width = size
  canvas.height = size
  const ctx = canvas.getContext('2d')!
  const cx = size / 2, cy = size / 2, r = size / 2

  // Outer halo (large, very soft)
  const halo = ctx.createRadialGradient(cx, cy, 0, cx, cy, r)
  halo.addColorStop(0,   hex + 'cc')
  halo.addColorStop(0.25, hex + '99')
  halo.addColorStop(0.55, hex + '44')
  halo.addColorStop(1,   hex + '00')
  ctx.fillStyle = halo
  ctx.fillRect(0, 0, size, size)

  // Core bright center
  const core = ctx.createRadialGradient(cx, cy, 0, cx, cy, r * 0.28)
  core.addColorStop(0,   '#ffffff')
  core.addColorStop(0.4, hex + 'ff')
  core.addColorStop(1,   hex + '00')
  ctx.fillStyle = core
  ctx.fillRect(0, 0, size, size)

  // Selection ring
  if (selected) {
    ctx.strokeStyle = '#ffffff'
    ctx.lineWidth = 6
    ctx.globalAlpha = 0.85
    ctx.beginPath()
    ctx.arc(cx, cy, r * 0.42, 0, Math.PI * 2)
    ctx.stroke()
    ctx.globalAlpha = 1
  }

  const tex = new CanvasTexture(canvas)
  textureCache.set(key, tex)
  return tex
}

function buildSprite(gnode: GNode, isSelected: boolean): Sprite {
  const col = gnode.soul_color || gnode.custom_color || NODE_COLORS[gnode.node_type] || '#40c8ff'

  // Convert named color or shorthand to full hex
  const tmp = document.createElement('canvas')
  tmp.width = tmp.height = 1
  const tmpCtx = tmp.getContext('2d')!
  tmpCtx.fillStyle = col
  tmpCtx.fillRect(0, 0, 1, 1)
  const [r, g, b] = tmpCtx.getImageData(0, 0, 1, 1).data
  const hex = '#' + [r, g, b].map(v => v.toString(16).padStart(2, '0')).join('')

  const alpha = gnode.is_ghost ? 0.3 : gnode.is_void ? 0.2 : 1.0
  const texture = makeGlowTexture(hex, alpha, isSelected)

  const mat = new SpriteMaterial({
    map: texture,
    transparent: true,
    opacity: alpha,
    blending: AdditiveBlending,  // ← makes nodes glow like light sources
    depthWrite: false,
  })

  const sprite = new Sprite(mat)

  // Size based on gravity/visual_weight
  const w = gnode.visual_weight ?? gnode.gravity
  const entropy = Math.max(0, Math.min(1, gnode.entropy))
  const baseSize = Math.max(8, Math.min(w * 22, 48))
  const shrink = gnode.is_ghost ? 0.6 : gnode.is_void ? 0.4 : 1 - entropy * 0.3
  const sz = baseSize * shrink * (isSelected ? 1.5 : 1)

  sprite.scale.set(sz, sz, 1)
  return sprite
}

function rgbaToHex(c: [number, number, number, number]) {
  const to = (v: number) => Math.round(Math.max(0, Math.min(1, v)) * 255).toString(16).padStart(2, '0')
  return `#${to(c[0])}${to(c[1])}${to(c[2])}`
}

function makeGNode(n: SNode, soulColor?: string): GNode {
  return {
    id: n.id, node_type: n.node_type, content: n.content,
    custom_color: n.custom_color || (n.node_type === 'other' ? n.aura_color : undefined),
    gravity: n.gravity, entropy: n.entropy, velocity: n.velocity,
    is_ghost: n.is_ghost, is_fossil: n.is_fossil, is_void: n.is_void,
    entropy_state: n.entropy_state, velocity_state: n.velocity_state,
    visual_weight: n.visual_weight, contagion_heat: n.contagion_heat,
    soul_color: soulColor,
    x: n.position.x * 120, y: n.position.y * 120, z: n.position.z * 120,
  }
}

export default function Graph3D({ nodes, edges, selectedId, onSelect, dim, trail = [], souls = [] }: Props) {
  const fgRef = useRef<any>(null)
  const [paused, setPaused] = useState(false)
  const [labelsEnabled, setLabelsEnabled] = useState(true)
  const nodeMapRef = useRef<Map<string, GNode>>(new Map())
  const [graphData, setGraphData] = useState<{ nodes: GNode[]; links: GLink[] }>({ nodes: [], links: [] })

  const soulColors = useMemo(() => {
    const map = new Map<string, string>()
    souls.forEach(s => map.set(s.project_id, rgbaToHex(s.primary_color)))
    return map
  }, [souls])

  const trailLinks = useMemo(() => {
    const recent = trail.slice(-18)
    const out: GLink[] = []
    for (let i = 1; i < recent.length; i++) {
      if (recent[i - 1].node_id !== recent[i].node_id) {
        out.push({
          source: recent[i - 1].node_id, target: recent[i].node_id,
          edge_type: 'focus_trail', weight: 0.7 + i / recent.length,
        })
      }
    }
    return out
  }, [trail])

  useEffect(() => {
    const nodeMap = nodeMapRef.current
    const incomingIds = new Set(nodes.map(n => n.id))

    nodeMap.forEach((_, id) => { if (!incomingIds.has(id)) nodeMap.delete(id) })

    nodes.forEach(n => {
      const sc = soulColors.get(n.id)
      const existing = nodeMap.get(n.id)
      if (existing) {
        existing.content = n.content; existing.gravity = n.gravity
        existing.entropy = n.entropy; existing.velocity = n.velocity
        existing.is_ghost = n.is_ghost; existing.is_fossil = n.is_fossil
        existing.is_void = n.is_void; existing.entropy_state = n.entropy_state
        existing.velocity_state = n.velocity_state; existing.visual_weight = n.visual_weight
        existing.contagion_heat = n.contagion_heat; existing.soul_color = sc
      } else {
        nodeMap.set(n.id, makeGNode(n, sc))
      }
    })

    const links: GLink[] = edges.map(e => ({
      source: e.source_id, target: e.target_id,
      edge_type: e.edge_type, weight: e.weight,
    })).concat(trailLinks)

    setGraphData({ nodes: Array.from(nodeMap.values()), links })
  }, [nodes, edges, soulColors, trailLinks])

  // Tune D3 forces for a more spread-out, organic layout
  useEffect(() => {
    if (!fgRef.current) return
    fgRef.current.d3Force('charge')?.strength(-280)
    fgRef.current.d3Force('link')?.distance(90).strength(0.5)
    fgRef.current.d3Force('center')?.strength(0.04)
  }, [graphData])

  const nodeThreeObject = useCallback((gnode: object) => {
    return buildSprite(gnode as GNode, (gnode as GNode).id === selectedId)
  }, [selectedId])

  const linkColor = useCallback((link: object) => {
    const l = link as GLink
    if (l.edge_type === 'resonance')   return 'rgba(139,92,246,0.6)'
    if (l.edge_type === 'temporal')    return 'rgba(251,191,36,0.5)'
    if (l.edge_type === 'causal')      return 'rgba(52,211,153,0.5)'
    if (l.edge_type === 'focus_trail') return 'rgba(45,212,191,0.9)'
    return 'rgba(64,180,255,0.25)'
  }, [])

  const linkWidth = useCallback((link: object) => {
    const l = link as GLink
    if (l.edge_type === 'focus_trail') return Math.max(1.2, l.weight * 2)
    return Math.max(0.4, l.weight * 1.4)
  }, [])

  const handleNodeClick = useCallback((gnode: object) => {
    const n = gnode as GNode
    onSelect(nodes.find(nd => nd.id === n.id) ?? null)
    if (fgRef.current && n.x != null) {
      fgRef.current.cameraPosition(
        { x: n.x + 80, y: (n.y ?? 0) + 40, z: (n.z ?? 0) + 80 },
        { x: n.x, y: n.y ?? 0, z: n.z ?? 0 },
        900,
      )
    }
  }, [nodes, onSelect])

  const centerGraph = useCallback(() => {
    if (!fgRef.current) return
    fgRef.current.zoomToFit?.(700, 80)
  }, [])

  const focusSelected = useCallback(() => {
    if (!fgRef.current || !selectedId) { centerGraph(); return }
    const node = nodeMapRef.current.get(selectedId)
    if (!node || node.x == null) return
    fgRef.current.cameraPosition(
      { x: node.x + 80, y: node.y! + 48, z: node.z! + 80 },
      { x: node.x, y: node.y!, z: node.z! },
      700,
    )
  }, [centerGraph, selectedId])

  const togglePaused = useCallback(() => {
    if (!fgRef.current) return
    if (paused) {
      fgRef.current.resumeAnimation?.()
      fgRef.current.d3ReheatSimulation?.()
      setPaused(false)
    } else {
      fgRef.current.pauseAnimation?.()
      setPaused(true)
    }
  }, [paused])

  useEffect(() => {
    if (fgRef.current) fgRef.current.cameraPosition({ x: 0, y: 0, z: 380 })
  }, [])

  useEffect(() => {
    if (selectedId) focusSelected()
  }, [focusSelected, selectedId])

  return (
    <div style={{ position: 'absolute', inset: 0, opacity: dim ? 0.25 : 1, transition: 'opacity 0.5s ease', zIndex: 0 }}>
      <div className="graph-controls">
        <button onClick={centerGraph}>Center</button>
        <button onClick={focusSelected} disabled={!selectedId}>Focus</button>
        <button onClick={togglePaused}>{paused ? 'Resume' : 'Pause'}</button>
        <button className={labelsEnabled ? 'active' : ''} onClick={() => setLabelsEnabled(v => !v)}>Labels</button>
        <button onClick={() => { fgRef.current?.resumeAnimation?.(); fgRef.current?.d3ReheatSimulation?.(); setPaused(false) }}>Drift</button>
      </div>

      <ForceGraph3D
        ref={fgRef}
        graphData={graphData}
        backgroundColor="rgba(0,0,0,0)"
        nodeLabel={(n: object) => labelsEnabled ? (n as GNode).content : ''}
        nodeThreeObject={nodeThreeObject}
        nodeThreeObjectExtend={false}
        linkColor={linkColor}
        linkWidth={linkWidth}
        linkOpacity={1}
        linkCurvature={0.12}
        linkDirectionalParticles={(l: object) => (l as GLink).edge_type === 'focus_trail' ? 4 : 0}
        linkDirectionalParticleWidth={(l: object) => (l as GLink).edge_type === 'focus_trail' ? 2.5 : 0}
        linkDirectionalParticleSpeed={0.005}
        onNodeClick={handleNodeClick}
        onBackgroundClick={() => onSelect(null)}
        enableNodeDrag={true}
        enableNavigationControls={true}
        showNavInfo={false}
        rendererConfig={{ antialias: true, alpha: true }}
        d3AlphaDecay={paused ? 1 : 0.018}
        d3VelocityDecay={paused ? 1 : 0.28}
        cooldownTime={paused ? 0 : 12000}
      />
    </div>
  )
}
