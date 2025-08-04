import { useEffect, useState } from 'react'
import './App.css'

interface OpcUaDriver {
  id: string
  name: string
  address: string
  connected: boolean
  driver_type: string
}

interface BrowseResult {
  node_id: string
  children: string[]
  error: string | null
}

export default function OpcUaBrowserPage() {
  const [drivers, setDrivers] = useState<OpcUaDriver[]>([])
  const [selectedDriver, setSelectedDriver] = useState<string>('')
  const [browseHistory, setBrowseHistory] = useState<string[]>(['ns=0;i=85'])
  const [currentNode, setCurrentNode] = useState<string>('ns=0;i=85')
  const [browseResult, setBrowseResult] = useState<BrowseResult | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const headers = { Authorization: 'Basic ' + btoa('admin:admin') }

  useEffect(() => {
    fetch('/api/opcua/discover', { headers })
      .then(res => {
        if (!res.ok) throw new Error('Failed to discover OPC UA drivers')
        return res.json()
      })
      .then(data => {
        setDrivers(data.drivers)
        if (data.drivers.length > 0) {
          setSelectedDriver(data.drivers[0].id)
        }
      })
      .catch(err => setError(err.message))
  }, [])

  const browseNode = async (nodeId: string) => {
    if (!selectedDriver) return
    
    setLoading(true)
    setError(null)
    
    try {
      const response = await fetch(`/api/opcua/browse/${selectedDriver}?node_id=${encodeURIComponent(nodeId)}`, { headers })
      if (!response.ok) throw new Error('Failed to browse node')
      
      const result: BrowseResult = await response.json()
      setBrowseResult(result)
      setCurrentNode(nodeId)
      
      if (result.error) {
        setError(result.error)
      }
    } catch (err: any) {
      setError(err.message)
    } finally {
      setLoading(false)
    }
  }

  const navigateToChild = (childName: string) => {
    // This is a simplified approach - in reality we'd need to construct proper node IDs
    // For now, we'll just try to browse common node patterns
    const childNodeId = `ns=2;s=${childName}`
    setBrowseHistory([...browseHistory, childNodeId])
    browseNode(childNodeId)
  }

  const navigateBack = () => {
    if (browseHistory.length > 1) {
      const newHistory = browseHistory.slice(0, -1)
      setBrowseHistory(newHistory)
      const previousNode = newHistory[newHistory.length - 1]
      browseNode(previousNode)
    }
  }

  const navigateToRoot = () => {
    setBrowseHistory(['ns=0;i=85'])
    browseNode('ns=0;i=85')
  }

  useEffect(() => {
    if (selectedDriver) {
      browseNode(currentNode)
    }
  }, [selectedDriver])

  return (
    <div className="App">
      <h1>OPC UA Tag Browser</h1>
      
      {error && <p className="error">{error}</p>}
      
      <div style={{ marginBottom: '20px' }}>
        <label>
          Select OPC UA Driver: 
          <select 
            value={selectedDriver} 
            onChange={(e) => setSelectedDriver(e.target.value)}
            style={{ marginLeft: '10px' }}
          >
            <option value="">-- Select Driver --</option>
            {drivers.map(driver => (
              <option key={driver.id} value={driver.id}>
                {driver.name} ({driver.connected ? 'Connected' : 'Disconnected'})
              </option>
            ))}
          </select>
        </label>
      </div>

      {selectedDriver && (
        <div>
          <div style={{ marginBottom: '20px' }}>
            <button onClick={navigateToRoot} disabled={loading}>Root</button>
            <button onClick={navigateBack} disabled={loading || browseHistory.length <= 1} style={{ marginLeft: '10px' }}>
              Back
            </button>
            <span style={{ marginLeft: '20px' }}>Current Node: {currentNode}</span>
          </div>

          {loading && <p>Loading...</p>}

          {browseResult && !loading && (
            <div>
              <h3>Children of {browseResult.node_id}</h3>
              {browseResult.children.length === 0 ? (
                <p>No children found</p>
              ) : (
                <table>
                  <thead>
                    <tr>
                      <th>Name</th>
                      <th>Actions</th>
                    </tr>
                  </thead>
                  <tbody>
                    {browseResult.children.map((child, index) => (
                      <tr key={index}>
                        <td>{child}</td>
                        <td>
                          <button onClick={() => navigateToChild(child)}>
                            Browse
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  )
}