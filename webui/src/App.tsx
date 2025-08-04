import { BrowserRouter as Router, Routes, Route, Link } from 'react-router-dom'
import Dashboard from './Dashboard'
import TagsPage from './TagsPage'
import OpcUaBrowserPage from './OpcUaBrowserPage'
import './App.css'

export default function App() {
  return (
    <Router>
      <nav className="nav">
        <Link to="/">Dashboard</Link> | <Link to="/tags">Tags</Link> | <Link to="/opcua-browser">OPC UA Browser</Link>
      </nav>
      <Routes>
        <Route path="/" element={<Dashboard />} />
        <Route path="/tags" element={<TagsPage />} />
        <Route path="/opcua-browser" element={<OpcUaBrowserPage />} />
      </Routes>
    </Router>
  )
}
