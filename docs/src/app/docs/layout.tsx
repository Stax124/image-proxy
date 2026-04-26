import { DocsLayout, type DocsLayoutProps } from 'fumadocs-ui/layouts/docs';
import { baseOptions } from '@/lib/layout.shared';
import { source } from '@/lib/source';
import { GithubInfo } from 'fumadocs-ui/components/github-info';

function docsOptions(): DocsLayoutProps {
  return {
    ...baseOptions(),
    tree: source.getPageTree(),
    links: [
      {
        type: 'custom',
        children: <GithubInfo owner="Stax124" repo="image-proxy" />,
      },
    ],
  };
}

export default function Layout({ children }: LayoutProps<'/docs'>) {
  return (
    <DocsLayout {...docsOptions()}>
      {children}
    </DocsLayout>
  );
}