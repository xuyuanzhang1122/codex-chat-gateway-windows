import { useEffect, useRef } from "react";

type Particle = {
  x: number;
  y: number;
  vx: number;
  vy: number;
  r: number;
  a: number;
};

type Props = {
  running: boolean;
};

const COUNT = 64;
const LINK = 115;

export function Particles({ running }: Props) {
  const ref = useRef<HTMLCanvasElement | null>(null);
  const runningRef = useRef(running);
  const mouse = useRef({ x: -9999, y: -9999 });

  useEffect(() => {
    runningRef.current = running;
  }, [running]);

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let raf = 0;
    let w = 0;
    let h = 0;
    let last = 0;
    const particles: Particle[] = [];

    const resize = () => {
      const parent = canvas.parentElement;
      w = parent?.clientWidth ?? window.innerWidth;
      h = parent?.clientHeight ?? window.innerHeight;
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      canvas.width = Math.floor(w * dpr);
      canvas.height = Math.floor(h * dpr);
      canvas.style.width = `${w}px`;
      canvas.style.height = `${h}px`;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };

    const spawn = (): Particle => {
      const speed = 6 + Math.random() * 14;
      const angle = Math.random() * Math.PI * 2;
      return {
        x: Math.random() * Math.max(1, w),
        y: Math.random() * Math.max(1, h),
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed,
        r: 1.2 + Math.random() * 2.2,
        a: 0.3 + Math.random() * 0.7,
      };
    };

    const onMove = (e: MouseEvent) => {
      const rect = canvas.getBoundingClientRect();
      mouse.current.x = e.clientX - rect.left;
      mouse.current.y = e.clientY - rect.top;
    };
    const onLeave = () => {
      mouse.current.x = -9999;
      mouse.current.y = -9999;
    };

    resize();
    for (let i = 0; i < COUNT; i++) particles.push(spawn());
    window.addEventListener("resize", resize);
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseleave", onLeave);

    const frame = (ts: number) => {
      const dt = Math.min(0.06, last ? (ts - last) / 1000 : 0.016);
      last = ts;
      const on = runningRef.current;
      const base = on ? { r: 0, g: 245, b: 212 } : { r: 110, g: 130, b: 150 };

      ctx.clearRect(0, 0, w, h);

      for (const p of particles) {
        p.x += p.vx * dt;
        p.y += p.vy * dt;
        const dx = p.x - mouse.current.x;
        const dy = p.y - mouse.current.y;
        const d2 = dx * dx + dy * dy;
        if (d2 < 140 * 140 && d2 > 0.01) {
          const d = Math.sqrt(d2);
          const push = ((140 - d) / 140) * 110 * dt;
          p.x += (dx / d) * push;
          p.y += (dy / d) * push;
        }
        if (p.x < -12) p.x = w + 10;
        else if (p.x > w + 12) p.x = -10;
        if (p.y < -12) p.y = h + 10;
        else if (p.y > h + 12) p.y = -10;
      }

      const link2 = LINK * LINK;
      for (let i = 0; i < COUNT; i++) {
        const a = particles[i];
        for (let j = i + 1; j < COUNT; j++) {
          const b = particles[j];
          const dx = a.x - b.x;
          const dy = a.y - b.y;
          const d2 = dx * dx + dy * dy;
          if (d2 >= link2 || d2 < 1) continue;
          const strength = 1 - Math.sqrt(d2) / LINK;
          ctx.strokeStyle = `rgba(${base.r},${base.g},${base.b},${0.08 + strength * 0.22})`;
          ctx.lineWidth = 1;
          ctx.beginPath();
          ctx.moveTo(a.x, a.y);
          ctx.lineTo(b.x, b.y);
          ctx.stroke();
        }
      }

      for (const p of particles) {
        const grad = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, p.r * 2.2);
        grad.addColorStop(0, `rgba(${base.r},${base.g},${base.b},${0.25 + p.a * 0.55})`);
        grad.addColorStop(1, `rgba(${base.r},${base.g},${base.b},0)`);
        ctx.fillStyle = grad;
        ctx.beginPath();
        ctx.arc(p.x, p.y, p.r * 2.2, 0, Math.PI * 2);
        ctx.fill();
      }

      raf = requestAnimationFrame(frame);
    };

    raf = requestAnimationFrame(frame);
    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", resize);
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseleave", onLeave);
    };
  }, []);

  return <canvas ref={ref} className="particles" aria-hidden />;
}
