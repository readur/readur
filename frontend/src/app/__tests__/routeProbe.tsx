import { useLocation, useParams } from 'react-router-dom';

/** Stand-in for a feature entry: names itself and shows where the router put us. */
export function probe(name: string) {
  function Probe() {
    const { pathname, search, hash } = useLocation();
    const params = useParams();
    return (
      <>
        <h1>{name} entry</h1>
        <output aria-label="location">
          {pathname}
          {search}
          {hash}
        </output>
        <output aria-label="params">{JSON.stringify(params)}</output>
      </>
    );
  }
  return Probe;
}
