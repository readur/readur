import React, { useState, useEffect } from 'react'
import {
  ChartBarIcon,
  ClockIcon,
  DocumentTextIcon,
  ExclamationCircleIcon,
} from '../design/icons'
import { Document } from '../services/api'

interface OcrAnalyticsProps {
  documents: Document[]
}

interface OcrStats {
  totalDocuments: number
  documentsWithOcr: number
  averageConfidence: number
  highConfidenceCount: number
  lowConfidenceCount: number
  failedCount: number
  processingCount: number
  totalWords: number
  averageProcessingTime: number
}

function OcrAnalytics({ documents }: OcrAnalyticsProps) {
  const [stats, setStats] = useState<OcrStats | null>(null)

  useEffect(() => {
    if (documents.length === 0) {
      setStats(null)
      return
    }

    const ocrDocuments = documents.filter(doc => doc.has_ocr_text)
    const completedOcr = ocrDocuments.filter(doc => doc.ocr_status === 'completed')
    const failedOcr = ocrDocuments.filter(doc => doc.ocr_status === 'failed')
    const processingOcr = ocrDocuments.filter(doc => doc.ocr_status === 'processing')
    
    const confidenceScores = completedOcr
      .map(doc => doc.ocr_confidence)
      .filter((confidence): confidence is number => confidence !== undefined)
    
    const wordCounts = completedOcr
      .map(doc => doc.ocr_word_count)
      .filter((count): count is number => count !== undefined)
    
    const processingTimes = completedOcr
      .map(doc => doc.ocr_processing_time_ms)
      .filter((time): time is number => time !== undefined)

    const averageConfidence = confidenceScores.length > 0 
      ? confidenceScores.reduce((sum, conf) => sum + conf, 0) / confidenceScores.length
      : 0

    const totalWords = wordCounts.reduce((sum, count) => sum + count, 0)
    
    const averageProcessingTime = processingTimes.length > 0
      ? processingTimes.reduce((sum, time) => sum + time, 0) / processingTimes.length
      : 0

    const highConfidenceCount = confidenceScores.filter(conf => conf >= 80).length
    const lowConfidenceCount = confidenceScores.filter(conf => conf < 60).length

    setStats({
      totalDocuments: documents.length,
      documentsWithOcr: ocrDocuments.length,
      averageConfidence,
      highConfidenceCount,
      lowConfidenceCount,
      failedCount: failedOcr.length,
      processingCount: processingOcr.length,
      totalWords,
      averageProcessingTime,
    })
  }, [documents])

  if (!stats || stats.documentsWithOcr === 0) {
    return null
  }

  const formatTime = (ms: number) => {
    if (ms < 1000) return `${Math.round(ms)}ms`
    return `${(ms / 1000).toFixed(1)}s`
  }

  const getConfidenceColor = (confidence: number) => {
    if (confidence >= 80) return 'text-green-600'
    if (confidence >= 60) return 'text-yellow-600'
    return 'text-orange-600'
  }

  const successRate = ((stats.documentsWithOcr - stats.failedCount) / stats.documentsWithOcr) * 100

  return (
    <div className="bg-white overflow-hidden shadow rounded-lg">
      <div className="p-5">
        <div className="flex items-center">
          <div className="flex-shrink-0">
            <ChartBarIcon className="h-6 w-6 text-gray-400" />
          </div>
          <div className="ml-5 w-0 flex-1">
            <dl>
              <dt className="text-sm font-medium text-gray-500 truncate">
                OCR Analytics
              </dt>
              <dd className="text-lg font-medium text-gray-900">
                {stats.documentsWithOcr} of {stats.totalDocuments} documents processed
              </dd>
            </dl>
          </div>
        </div>
      </div>
      
      <div className="bg-gray-50 px-5 py-3">
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
          {/* Success Rate */}
          <div className="text-center">
            <div className="text-lg font-semibold text-gray-900">
              {successRate.toFixed(0)}%
            </div>
            <div className="text-xs text-gray-500">Success Rate</div>
          </div>

          {/* Average Confidence */}
          <div className="text-center">
            <div className={`text-lg font-semibold ${getConfidenceColor(stats.averageConfidence)}`}>
              {stats.averageConfidence.toFixed(0)}%
            </div>
            <div className="text-xs text-gray-500">Avg Confidence</div>
          </div>

          {/* Total Words */}
          <div className="text-center">
            <div className="text-lg font-semibold text-gray-900">
              {stats.totalWords.toLocaleString()}
            </div>
            <div className="text-xs text-gray-500">Words Extracted</div>
          </div>

          {/* Average Processing Time */}
          <div className="text-center">
            <div className="text-lg font-semibold text-gray-900">
              {formatTime(stats.averageProcessingTime)}
            </div>
            <div className="text-xs text-gray-500">Avg Time</div>
          </div>
        </div>

        {/* Quality Distribution */}
        <div className="mt-4 pt-4 border-t border-gray-200">
          <div className="flex justify-between items-center text-sm">
            <div className="flex items-center space-x-4">
              <div className="flex items-center">
                <div className="w-2 h-2 bg-green-500 rounded-full mr-1"></div>
                <span className="text-gray-600">High Quality: {stats.highConfidenceCount}</span>
              </div>
              
              {stats.lowConfidenceCount > 0 && (
                <div className="flex items-center">
                  <div className="w-2 h-2 bg-orange-500 rounded-full mr-1"></div>
                  <span className="text-gray-600">Low Quality: {stats.lowConfidenceCount}</span>
                </div>
              )}
              
              {stats.failedCount > 0 && (
                <div className="flex items-center">
                  <div className="w-2 h-2 bg-red-500 rounded-full mr-1"></div>
                  <span className="text-gray-600">Failed: {stats.failedCount}</span>
                </div>
              )}
              
              {stats.processingCount > 0 && (
                <div className="flex items-center">
                  <div className="w-2 h-2 bg-yellow-500 rounded-full mr-1 animate-pulse"></div>
                  <span className="text-gray-600">Processing: {stats.processingCount}</span>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export default OcrAnalytics