import { render } from '@testing-library/react';
import ConfidenceRing from '../components/ui/ConfidenceRing';

describe('ConfidenceRing', () => {
  it('renders a decorative SVG with aria-hidden="true" and default size 22', () => {
    const { container } = render(<ConfidenceRing value={0.98} />);
    const svg = container.querySelector('svg');
    expect(svg).toBeInTheDocument();
    expect(svg).toHaveAttribute('aria-hidden', 'true');
    expect(svg).toHaveAttribute('width', '22');
    expect(svg).toHaveAttribute('height', '22');
    expect(svg).toHaveAttribute('viewBox', '0 0 22 22');
    expect(svg).toHaveClass('k-ring', 'k-ring--solid');
  });

  it('renders solid class for healthy confidence (>= 0.95)', () => {
    const { container: c1 } = render(<ConfidenceRing value={0.98} />);
    expect(c1.querySelector('svg')).toHaveClass('k-ring--solid');

    const { container: c2 } = render(<ConfidenceRing value={0.95} />);
    expect(c2.querySelector('svg')).toHaveClass('k-ring--solid');
  });

  it('renders building class for review confidence (< 0.95)', () => {
    const { container: c1 } = render(<ConfidenceRing value={0.85} />);
    expect(c1.querySelector('svg')).toHaveClass('k-ring--building');

    const { container: c2 } = render(<ConfidenceRing value={0.3} />);
    expect(c2.querySelector('svg')).toHaveClass('k-ring--building');
  });

  it('renders unseen class and no arc for null or undefined value', () => {
    const { container: cNull } = render(<ConfidenceRing value={null} />);
    const svgNull = cNull.querySelector('svg');
    expect(svgNull).toHaveClass('k-ring--unseen');
    expect(cNull.querySelector('.k-ring__track')).toBeInTheDocument();
    expect(cNull.querySelector('.k-ring__arc')).not.toBeInTheDocument();

    const { container: cUndef } = render(<ConfidenceRing value={undefined} />);
    const svgUndef = cUndef.querySelector('svg');
    expect(svgUndef).toHaveClass('k-ring--unseen');
    expect(cUndef.querySelector('.k-ring__track')).toBeInTheDocument();
    expect(cUndef.querySelector('.k-ring__arc')).not.toBeInTheDocument();
  });

  it('supports custom sizes (e.g. 18, 20, 34)', () => {
    const { container: c18 } = render(<ConfidenceRing value={0.85} size={18} />);
    const svg18 = c18.querySelector('svg');
    expect(svg18).toHaveAttribute('width', '18');
    expect(svg18).toHaveAttribute('height', '18');
    expect(svg18).toHaveAttribute('viewBox', '0 0 18 18');

    const { container: c34 } = render(<ConfidenceRing value={0.98} size={34} />);
    const svg34 = c34.querySelector('svg');
    expect(svg34).toHaveAttribute('width', '34');
    expect(svg34).toHaveAttribute('height', '34');
    expect(svg34).toHaveAttribute('viewBox', '0 0 34 34');
  });

  it('computes expected geometry for the arc and track', () => {
    const size = 22;
    const sw = Math.max(2.2, size * 0.13); // 2.86
    const r = (size - sw) / 2; // (22 - 2.86) / 2 = 9.57
    const c = size / 2; // 11
    const len = 2 * Math.PI * r;
    const v = 0.5;

    const { container } = render(<ConfidenceRing value={v} size={size} />);
    const track = container.querySelector('.k-ring__track');
    expect(track).toHaveAttribute('cx', String(c));
    expect(track).toHaveAttribute('cy', String(c));
    expect(track).toHaveAttribute('r', String(r));
    expect(track).toHaveAttribute('stroke-width', String(sw));

    const arc = container.querySelector('.k-ring__arc');
    expect(arc).toBeInTheDocument();
    expect(arc).toHaveAttribute('cx', String(c));
    expect(arc).toHaveAttribute('cy', String(c));
    expect(arc).toHaveAttribute('r', String(r));
    expect(arc).toHaveAttribute('stroke-width', String(sw));
    expect(arc).toHaveAttribute('stroke-dasharray', `${(len * v).toFixed(2)} ${len.toFixed(2)}`);
    expect(arc).toHaveAttribute('transform', `rotate(-90 ${c} ${c})`);
  });

  it('does not render arc when value is 0', () => {
    const { container } = render(<ConfidenceRing value={0} />);
    expect(container.querySelector('.k-ring__track')).toBeInTheDocument();
    expect(container.querySelector('.k-ring__arc')).not.toBeInTheDocument();
  });
});
