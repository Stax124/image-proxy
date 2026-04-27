'use client';

import Link from 'next/link';
import { useState, useCallback } from 'react';
import { Zap, Image, BookOpenText, Copy, Check, Scaling } from 'lucide-react';

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, [text]);

  return (
    <button
      type="button"
      className="p-2 text-fd-muted-foreground hover:text-fd-foreground transition-colors cursor-pointer"
      onClick={handleCopy}
      aria-label={copied ? 'Copied' : 'Copy to clipboard'}
    >
      {copied ? (
        <Check className="size-4 text-green-500" />
      ) : (
        <Copy className="size-4" />
      )}
    </button>
  );
}

const features = [
  {
    icon: <Zap className="size-6 text-blue-600" />,
    title: 'Ultra Fast',
    description:
      'Built from the ground up in Rust for maximum performance and low latency. Safely handles high concurrent loads without breaking a sweat.',
  },
  {
    icon: <Image className="size-6 text-blue-600" />,
    title: 'Next-gen Formats',
    description:
      "Automatically convert images to modern formats like AVIF, WebP, and JPEG XL based on the client's 'Accept' header or specify a format directly in the URL.",
  },
  {
    icon: <Scaling className="size-6 text-blue-600" />,
    title: 'Dynamic Resizing',
    description:
      'Perform on-the-fly resizing using a straightforward URL API. No complex configuration required.',
  },
];

const dockerCommand =
  'docker run -p 8000:8000 -v /path/to/images:/app/data stax124/image-proxy';

export default function HomePage() {
  return (
    <div className="flex flex-col items-center flex-1 px-4 py-16 mt-24">

      {/* Hero */}
      <h1 className="max-w-2xl text-center text-4xl font-bold leading-tight tracking-tight md:text-5xl">
        Fast, efficient, and flexible{' '}
        <span className="text-blue-600">image transformation proxy</span>{' '}
        written in Rust
      </h1>
      <p className="mt-8 max-w-3xl text-center text-fd-muted-foreground">
        Serve perfectly optimized images to your users instantly. Resize
        and convert formats on-the-fly with a simple URL structure, backed by a
        robust LRU cache.
      </p>


      {/* CTA Buttons */}
      <div className="mt-8 flex flex-wrap items-center justify-center gap-3 select-none">
        <Link
          href="/docs"
          className="rounded-lg bg-blue-600 px-5 py-2.5 text-sm font-medium text-white hover:bg-blue-700 transition-colors inline-flex items-center gap-1"
        >
          <BookOpenText className="size-4 inline-block mr-1" />
          Read Docs
        </Link>
        <Link
          href="https://github.com/Stax124/image-proxy"
          target='_blank'
          rel="noopener noreferrer"
          className="inline-flex items-center gap-2 rounded-lg border border-fd-border bg-fd-card px-5 py-2.5 text-sm font-medium hover:bg-fd-accent transition-colors"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="currentColor"
          >
            <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
          </svg>
          View on GitHub
        </Link>
      </div>

      {/* Docker Command */}
      <div className="mt-10 flex w-full max-w-xl items-center rounded-xl border border-fd-border bg-fd-background/65 px-4 py-3">
        <code className="flex-1 text-sm text-fd-muted-foreground overflow-x-auto">
          <span className="text-blue-400 select-none mr-2">$</span>{dockerCommand}
        </code>
        <CopyButton text={dockerCommand} />
      </div>

      {/* Features Section */}
      <h2 className="mt-20 text-2xl font-semibold">
        Engineered for Performance
      </h2>
      <div className="mt-8 grid w-full max-w-7xl gap-6 lg:grid-cols-3">
        {features.map((feature) => (
          <div
            key={feature.title}
            className="rounded-xl bg-fd-background/65 border border-fd-border p-6"
          >
            <div className="mb-4 inline-flex rounded-lg border border-fd-border bg-fd-background p-3">
              {feature.icon}
            </div>
            <h3 className="text-lg font-semibold">{feature.title}</h3>
            <p className="mt-2 text-sm text-fd-muted-foreground">
              {feature.description}
            </p>
          </div>
        ))}
      </div>
    </div>
  );
}
