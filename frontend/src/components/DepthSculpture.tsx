// 3D liquidity view in the Bookmap tradition: live depth as glowing columns on a
// dark grid — green bids west of mid, red asks east, amber sparks on trades.
// Subscribes to the feed directly so React never re-renders the scene.

import { useEffect, useRef } from 'react'
import * as THREE from 'three'
import { feed } from '../lib/feed'

const DEPTH = 16
const SLAB_W = 0.62
const GAP = 0.1
const H_SCALE = 0.028
const MAX_H = 5.2
const SPARK_LIFE = 1.0

const BG = 0x0b0e13
const UP = 0x0aa574
const DOWN = 0xe5484d
const AMBER = 0xffb224

interface Spark {
  mesh: THREE.Mesh
  vel: THREE.Vector3
  age: number
}

function makeSlabs(scene: THREE.Scene, color: number, dir: 1 | -1): THREE.Mesh[] {
  const mat = new THREE.MeshStandardMaterial({
    color,
    roughness: 0.4,
    metalness: 0.1,
    transparent: true,
    opacity: 0.85,
    emissive: color,
    emissiveIntensity: 0.25,
  })
  const geo = new THREE.BoxGeometry(SLAB_W, 1, 1.5)
  return Array.from({ length: DEPTH }, (_, i) => {
    const mesh = new THREE.Mesh(geo, mat.clone())
    mesh.position.x = dir * (0.6 + i * (SLAB_W + GAP))
    mesh.scale.y = 0.001
    scene.add(mesh)
    return mesh
  })
}

export function DepthSculpture() {
  const mountRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const mount = mountRef.current
    if (!mount) return

    const scene = new THREE.Scene()
    scene.background = new THREE.Color(BG)
    scene.fog = new THREE.Fog(BG, 18, 38)

    const camera = new THREE.PerspectiveCamera(38, 1, 0.1, 100)
    const renderer = new THREE.WebGLRenderer({ antialias: true })
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
    mount.appendChild(renderer.domElement)

    scene.add(new THREE.HemisphereLight(0x334155, 0x0b0e13, 1.4))
    const key = new THREE.DirectionalLight(0xdbeafe, 1.2)
    key.position.set(6, 12, 8)
    scene.add(key)

    const grid = new THREE.GridHelper(44, 44, 0x232a35, 0x151a23)
    grid.position.y = -0.01
    scene.add(grid)

    // Mid-price marker: thin amber pillar at x = 0.
    const pillar = new THREE.Mesh(
      new THREE.CylinderGeometry(0.02, 0.02, 7, 6),
      new THREE.MeshBasicMaterial({ color: AMBER, transparent: true, opacity: 0.5 }),
    )
    pillar.position.y = 3.4
    scene.add(pillar)

    const bidSlabs = makeSlabs(scene, UP, -1)
    const askSlabs = makeSlabs(scene, DOWN, 1)
    const bidTargets = new Float32Array(DEPTH)
    const askTargets = new Float32Array(DEPTH)

    const sparkGeo = new THREE.SphereGeometry(0.08, 8, 8)
    const sparkMat = new THREE.MeshBasicMaterial({ color: AMBER, transparent: true })
    let sparks: Spark[] = []

    const unsub = feed.subscribe((msg) => {
      if (msg.type === 'book') {
        for (let i = 0; i < DEPTH; i++) {
          bidTargets[i] = Math.min(MAX_H, (msg.bids[i]?.quantity ?? 0) * H_SCALE)
          askTargets[i] = Math.min(MAX_H, (msg.asks[i]?.quantity ?? 0) * H_SCALE)
        }
      } else if (msg.type === 'trade' && sparks.length < 40) {
        const mesh = new THREE.Mesh(sparkGeo, sparkMat.clone())
        mesh.position.set(0, 0.4, 0)
        scene.add(mesh)
        sparks.push({
          mesh,
          vel: new THREE.Vector3(
            (Math.random() - 0.5) * 2.2,
            2.2 + Math.random() * 2,
            (Math.random() - 0.5) * 2.2,
          ),
          age: 0,
        })
      }
    })

    const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches
    const clock = new THREE.Clock()
    let raf = 0

    const resize = () => {
      renderer.setSize(mount.clientWidth, mount.clientHeight)
      camera.aspect = mount.clientWidth / mount.clientHeight
      camera.updateProjectionMatrix()
    }
    resize()
    const ro = new ResizeObserver(resize)
    ro.observe(mount)

    const animate = () => {
      raf = requestAnimationFrame(animate)
      const dt = Math.min(clock.getDelta(), 0.05)
      const t = clock.elapsedTime

      for (let i = 0; i < DEPTH; i++) {
        const b = bidSlabs[i]
        const a = askSlabs[i]
        b.scale.y += (Math.max(bidTargets[i], 0.001) - b.scale.y) * 0.12
        a.scale.y += (Math.max(askTargets[i], 0.001) - a.scale.y) * 0.12
        b.position.y = b.scale.y / 2
        a.position.y = a.scale.y / 2
      }

      sparks = sparks.filter((s) => {
        s.age += dt
        s.vel.y -= 6 * dt
        s.mesh.position.addScaledVector(s.vel, dt)
        const mat = s.mesh.material as THREE.MeshBasicMaterial
        mat.opacity = Math.max(0, 1 - s.age / SPARK_LIFE)
        if (s.age >= SPARK_LIFE) {
          scene.remove(s.mesh)
          mat.dispose()
          return false
        }
        return true
      })

      const orbit = reduceMotion ? 0.6 : t * 0.1
      camera.position.set(
        Math.sin(orbit) * 15,
        6.4 + Math.sin(t * 0.2) * 0.4,
        Math.cos(orbit) * 15,
      )
      camera.lookAt(0, 1.5, 0)
      renderer.render(scene, camera)
    }
    animate()

    return () => {
      unsub()
      cancelAnimationFrame(raf)
      ro.disconnect()
      renderer.dispose()
      mount.removeChild(renderer.domElement)
    }
  }, [])

  return (
    <div className="tpanel relative h-full min-h-0 overflow-hidden">
      <div className="pointer-events-none absolute left-3 top-2 z-10 flex w-[calc(100%-1.5rem)] items-center justify-between">
        <span className="tlabel">3D liquidity</span>
        <span className="text-[10px] text-text-3">amber = trades at mid</span>
      </div>
      <div ref={mountRef} className="h-full w-full" />
    </div>
  )
}
