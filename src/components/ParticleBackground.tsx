import { type Component, onCleanup, onMount } from "solid-js";

interface Particle {
  x: number;
  y: number;
  vx: number;
  vy: number;
  r: number;
}

const PARTICLE_COUNT = 55;
const MAX_DIST = 130;
const REPULSE_DIST = 90;
const REPULSE_FORCE = 0.018;
const SPEED = 0.35;

export const ParticleBackground: Component = () => {
  let canvas: HTMLCanvasElement | undefined;
  let raf: number;
  let mouse = { x: -9999, y: -9999 };

  onMount(() => {
    if (!canvas) return;

    // Respect prefers-reduced-motion
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    const ctx = canvas.getContext("2d")!;
    let w = 0, h = 0;
    let particles: Particle[] = [];

    function resize() {
      w = canvas!.width = canvas!.offsetWidth;
      h = canvas!.height = canvas!.offsetHeight;
    }

    function spawn(): Particle {
      const angle = Math.random() * Math.PI * 2;
      return {
        x: Math.random() * w,
        y: Math.random() * h,
        vx: Math.cos(angle) * SPEED * (0.4 + Math.random() * 0.6),
        vy: Math.sin(angle) * SPEED * (0.4 + Math.random() * 0.6),
        r: 1.5 + Math.random() * 1.5,
      };
    }

    function init() {
      resize();
      particles = Array.from({ length: PARTICLE_COUNT }, spawn);
    }

    function draw() {
      ctx.clearRect(0, 0, w, h);

      // Update positions
      for (const p of particles) {
        // Repulsion from mouse
        const dx = p.x - mouse.x;
        const dy = p.y - mouse.y;
        const dist = Math.sqrt(dx * dx + dy * dy);
        if (dist < REPULSE_DIST && dist > 0) {
          const force = (1 - dist / REPULSE_DIST) * REPULSE_FORCE;
          p.vx += (dx / dist) * force * 3;
          p.vy += (dy / dist) * force * 3;
        }

        // Dampen velocity to avoid runaway
        const speed = Math.sqrt(p.vx * p.vx + p.vy * p.vy);
        if (speed > SPEED * 3) {
          p.vx = (p.vx / speed) * SPEED * 3;
          p.vy = (p.vy / speed) * SPEED * 3;
        }

        p.x += p.vx;
        p.y += p.vy;

        // Bounce off edges
        if (p.x < 0) { p.x = 0; p.vx = Math.abs(p.vx); }
        if (p.x > w) { p.x = w; p.vx = -Math.abs(p.vx); }
        if (p.y < 0) { p.y = 0; p.vy = Math.abs(p.vy); }
        if (p.y > h) { p.y = h; p.vy = -Math.abs(p.vy); }
      }

      // Draw connections
      for (let i = 0; i < particles.length; i++) {
        for (let j = i + 1; j < particles.length; j++) {
          const a = particles[i], b = particles[j];
          const dx = a.x - b.x, dy = a.y - b.y;
          const d = Math.sqrt(dx * dx + dy * dy);
          if (d < MAX_DIST) {
            ctx.beginPath();
            ctx.strokeStyle = `rgba(100,140,220,${(1 - d / MAX_DIST) * 0.35})`;
            ctx.lineWidth = 0.8;
            ctx.moveTo(a.x, a.y);
            ctx.lineTo(b.x, b.y);
            ctx.stroke();
          }
        }
      }

      // Draw particles
      for (const p of particles) {
        ctx.beginPath();
        ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2);
        ctx.fillStyle = "rgba(120,160,255,0.55)";
        ctx.fill();
      }

      raf = requestAnimationFrame(draw);
    }

    const onMouse = (e: MouseEvent) => {
      const rect = canvas!.getBoundingClientRect();
      mouse = { x: e.clientX - rect.left, y: e.clientY - rect.top };
    };
    const onLeave = () => { mouse = { x: -9999, y: -9999 }; };

    const ro = new ResizeObserver(resize);
    ro.observe(canvas);
    canvas.addEventListener("mousemove", onMouse);
    canvas.addEventListener("mouseleave", onLeave);
    window.addEventListener("mousemove", onMouse);

    init();
    raf = requestAnimationFrame(draw);

    onCleanup(() => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      canvas?.removeEventListener("mousemove", onMouse);
      canvas?.removeEventListener("mouseleave", onLeave);
      window.removeEventListener("mousemove", onMouse);
    });
  });

  return (
    <canvas
      ref={canvas}
      style={{
        position: "fixed",
        inset: "0",
        width: "100%",
        height: "100%",
        "z-index": "0",
        "pointer-events": "none",
        opacity: "0.4",
      }}
    />
  );
};
