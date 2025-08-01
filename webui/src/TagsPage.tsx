import { useEffect, useState } from 'react'
import './App.css'

function displayValue(val: unknown) {
  if (val && typeof val === 'object') {
    const [type, v] = Object.entries(val)[0]
    return `${v}`
  }
  return val
}

export default function TagsPage() {
  const [tags, setTags] = useState<any[]>([])
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    fetch('/tags')
      .then(res => {
        if (!res.ok) throw new Error('Failed to fetch tags')
        return res.json()
      })
      .then(setTags)
      .catch(err => setError(err.message))
  }, [])

  return (
    <div className="App">
      <h1>Tag Monitor</h1>
      {error && <p className="error">{error}</p>}
      <table>
        <thead>
          <tr>
            <th>Path</th>
            <th>Value</th>
            <th>Quality</th>
            <th>Timestamp</th>
          </tr>
        </thead>
        <tbody>
          {tags.map(tag => (
            <tr key={tag.path}>
              <td>{tag.path}</td>
              <td>{displayValue(tag.value.value)}</td>
              <td>{tag.value.quality}</td>
              <td>{new Date(tag.value.timestamp).toLocaleString()}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
