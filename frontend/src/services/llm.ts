import axios from 'axios';

const API_URL = '/api/llm';

export interface GraphNode {
    id: string;
    label: string;
    name: string;
    properties: any;
}

export interface GraphEdge {
    id: string;
    source: string;
    target: string;
    relationship: string;
    properties: any;
}

export interface GraphData {
    nodes: GraphNode[];
    edges: GraphEdge[];
}

export const analyzeDocument = async (documentId: string): Promise<GraphData> => {
    const response = await axios.post(`${API_URL}/${documentId}/analyze`);
    return response.data;
};

export const getDocumentGraph = async (documentId: string): Promise<GraphData> => {
    const response = await axios.get(`${API_URL}/${documentId}/graph`);
    return response.data;
};
