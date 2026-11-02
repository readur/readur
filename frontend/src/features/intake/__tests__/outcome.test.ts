import { describe, expect, it } from 'vitest';
import { bulkDeleteResult, outcomeOf } from '../attention/outcome';

describe('outcomeOf', () => {
  it('reads a count against the number requested', () => {
    expect(outcomeOf(3, 3)).toBe('all');
    expect(outcomeOf(1, 3)).toBe('some');
    expect(outcomeOf(0, 3)).toBe('none');
    expect(outcomeOf(undefined, 3)).toBe('none');
  });
});

describe('bulkDeleteResult', () => {
  it('counts only the ids the server lists as deleted', () => {
    const r = bulkDeleteResult({ deleted_count: 1, failed_count: 1, deleted_documents: ['a'] }, ['a', 'b']);
    expect(r.outcome).toBe('some');
    expect(r.deleted).toBe(1);
    expect([...r.gone]).toEqual(['a']);
  });

  it('treats a zero count as nothing deleted', () => {
    const r = bulkDeleteResult({ deleted_count: 0, failed_count: 2, deleted_documents: [] }, ['a', 'b']);
    expect(r.outcome).toBe('none');
    expect(r.gone.size).toBe(0);
  });

  it('without a list, everything is gone only when the count covers every id', () => {
    expect([...bulkDeleteResult({ deleted_count: 2 }, ['a', 'b']).gone]).toEqual(['a', 'b']);
    expect(bulkDeleteResult({ deleted_count: 1 }, ['a', 'b']).gone.size).toBe(0);
    expect(bulkDeleteResult(undefined, ['a']).outcome).toBe('none');
  });

  it('ignores listed ids that were not requested', () => {
    expect([...bulkDeleteResult({ deleted_count: 1, deleted_documents: ['zzz'] }, ['a']).gone]).toEqual([]);
  });
});
