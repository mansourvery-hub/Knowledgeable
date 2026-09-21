import { fireEvent, render, screen } from '@testing-library/react';
import SearchField from '../components/ui/SearchField';
import Button from '../components/ui/Button';
import Segmented from '../components/ui/Segmented';
import ConceptRow from '../components/ui/ConceptRow';
import EmptyState from '../components/ui/EmptyState';
import ErrorState from '../components/ui/ErrorState';

describe('SearchField', () => {
  it('renders k-field label with k-input and Search icon button', () => {
    render(
      <SearchField
        value=""
        onChange={() => {}}
        placeholder="Find a concept"
        ariaLabel="Find a concept"
        testId="graph-search-input"
      />,
    );
    const label = document.querySelector('label.k-field');
    expect(label).toBeInTheDocument();
    const input = screen.getByTestId('graph-search-input');
    expect(input).toHaveClass('k-input');
    expect(input).toHaveAttribute('placeholder', 'Find a concept');
    expect(input).toHaveAttribute('aria-label', 'Find a concept');
    const button = screen.getByRole('button', { name: 'Search' });
    expect(button).toHaveAttribute('data-testid', 'graph-search');
  });

  it('calls onChange while typing and onSubmit on Enter or icon click', () => {
    const onChange = jest.fn();
    const onSubmit = jest.fn();
    render(
      <SearchField value="a" onChange={onChange} testId="wiki-search-input" onSubmit={onSubmit} />,
    );
    fireEvent.change(screen.getByTestId('wiki-search-input'), { target: { value: 'ab' } });
    expect(onChange).toHaveBeenCalledWith('ab');
    fireEvent.keyDown(screen.getByTestId('wiki-search-input'), { key: 'Enter' });
    expect(onSubmit).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole('button', { name: 'Search' }));
    expect(onSubmit).toHaveBeenCalledTimes(2);
  });
});

describe('Button', () => {
  it('defaults to type button with k-btn class', () => {
    render(<Button>Open notes</Button>);
    const button = screen.getByRole('button', { name: 'Open notes' });
    expect(button).toHaveAttribute('type', 'button');
    expect(button).toHaveClass('k-btn');
  });

  it('applies primary and ghost variants', () => {
    const { rerender } = render(<Button variant="primary">Go</Button>);
    expect(screen.getByRole('button', { name: 'Go' })).toHaveClass('k-btn--primary');
    rerender(<Button variant="ghost">Back</Button>);
    expect(screen.getByRole('button', { name: 'Back' })).toHaveClass('k-btn--ghost');
  });
});

describe('Segmented', () => {
  it('renders All and Needs review counts with aria-pressed', () => {
    const onSelect = jest.fn();
    render(<Segmented allCount={3} reviewCount={2} selected="all" onSelect={onSelect} />);
    const group = screen.getByRole('group', { name: 'Filter' });
    expect(group).toHaveClass('k-seg');
    expect(screen.getByRole('button', { name: 'All 3' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: 'Needs review 2' })).toHaveAttribute(
      'aria-pressed',
      'false',
    );
    fireEvent.click(screen.getByRole('button', { name: 'Needs review 2' }));
    expect(onSelect).toHaveBeenCalledWith('review');
  });
});

describe('ConceptRow', () => {
  it('renders ring, name, and percent inside a list item', () => {
    render(
      <ul>
        <ConceptRow name="Factor" value={0.3} testId="concept-row" />
      </ul>,
    );
    const row = screen.getByTestId('concept-row');
    expect(row).toHaveClass('k-row');
    expect(row).toHaveAttribute('type', 'button');
    expect(row.querySelector('svg.k-ring')).toBeInTheDocument();
    expect(row).toHaveTextContent('Factor');
    expect(row).toHaveTextContent('30%');
    expect(row).not.toHaveAttribute('aria-current');
  });

  it('marks the selected concept with aria-current and renders sub', () => {
    const onSelect = jest.fn();
    render(
      <ul>
        <ConceptRow
          name="Divisibility"
          value={0.85}
          selected
          sub="May be outdated"
          onSelect={onSelect}
          testId="concept-row-selected"
        />
      </ul>,
    );
    const row = screen.getByTestId('concept-row-selected');
    expect(row).toHaveAttribute('aria-current', 'true');
    expect(row).toHaveTextContent('May be outdated');
    fireEvent.click(row);
    expect(onSelect).toHaveBeenCalled();
  });
});

describe('EmptyState', () => {
  it('renders the k-empty message', () => {
    render(<EmptyState message="No concepts match that search." testId="empty" />);
    const node = screen.getByTestId('empty');
    expect(node).toHaveClass('k-empty');
    expect(node).toHaveTextContent('No concepts match that search.');
  });
});

describe('ErrorState', () => {
  it('renders role alert with a retry Button', () => {
    const onRetry = jest.fn();
    render(
      <ErrorState
        message="Couldn't load the map. Try again."
        onRetry={onRetry}
        testId="map-error"
        retryTestId="map-retry"
      />,
    );
    expect(screen.getByRole('alert')).toHaveTextContent("Couldn't load the map. Try again.");
    fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(onRetry).toHaveBeenCalled();
  });
});
