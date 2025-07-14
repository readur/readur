import React, { useState, useEffect } from 'react'
import { CheckIcon, XMarkIcon } from '@heroicons/react/24/outline'
import { LanguageInfo } from '../services/api'

interface LanguageSelectorProps {
  selectedLanguages: string[]
  primaryLanguage?: string
  onLanguagesChange: (languages: string[], primary?: string) => void
  maxLanguages?: number
  disabled?: boolean
  showPrimarySelector?: boolean
  className?: string
}

// Common languages with display names
const COMMON_LANGUAGES: LanguageInfo[] = [
  { code: 'eng', name: 'English', installed: true },
  { code: 'spa', name: 'Spanish', installed: true },
  { code: 'fra', name: 'French', installed: true },
  { code: 'deu', name: 'German', installed: true },
  { code: 'ita', name: 'Italian', installed: true },
  { code: 'por', name: 'Portuguese', installed: true },
  { code: 'rus', name: 'Russian', installed: true },
  { code: 'chi_sim', name: 'Chinese (Simplified)', installed: true },
  { code: 'chi_tra', name: 'Chinese (Traditional)', installed: true },
  { code: 'jpn', name: 'Japanese', installed: true },
  { code: 'kor', name: 'Korean', installed: true },
  { code: 'ara', name: 'Arabic', installed: true },
  { code: 'hin', name: 'Hindi', installed: true },
  { code: 'nld', name: 'Dutch', installed: true },
  { code: 'swe', name: 'Swedish', installed: true },
  { code: 'nor', name: 'Norwegian', installed: true },
  { code: 'dan', name: 'Danish', installed: true },
  { code: 'fin', name: 'Finnish', installed: true },
  { code: 'pol', name: 'Polish', installed: true },
  { code: 'ces', name: 'Czech', installed: true },
  { code: 'hun', name: 'Hungarian', installed: true },
  { code: 'tur', name: 'Turkish', installed: true },
  { code: 'tha', name: 'Thai', installed: true },
  { code: 'vie', name: 'Vietnamese', installed: true },
]

function LanguageSelector({
  selectedLanguages,
  primaryLanguage,
  onLanguagesChange,
  maxLanguages = 4,
  disabled = false,
  showPrimarySelector = true,
  className = '',
}: LanguageSelectorProps) {
  const [availableLanguages, setAvailableLanguages] = useState<LanguageInfo[]>(COMMON_LANGUAGES)
  const [isOpen, setIsOpen] = useState(false)

  // Auto-set primary language to first selected if not specified
  const effectivePrimary = primaryLanguage || selectedLanguages[0] || ''

  const handleLanguageToggle = (languageCode: string) => {
    if (disabled) return

    let newLanguages: string[]
    let newPrimary = effectivePrimary

    if (selectedLanguages.includes(languageCode)) {
      // Remove language
      newLanguages = selectedLanguages.filter(lang => lang !== languageCode)
      // If removing the primary language, set new primary to first remaining language
      if (languageCode === effectivePrimary && newLanguages.length > 0) {
        newPrimary = newLanguages[0]
      } else if (newLanguages.length === 0) {
        newPrimary = ''
      }
    } else {
      // Add language (check max limit)
      if (selectedLanguages.length >= maxLanguages) {
        return
      }
      newLanguages = [...selectedLanguages, languageCode]
      // If this is the first language, make it primary
      if (newLanguages.length === 1) {
        newPrimary = languageCode
      }
    }

    onLanguagesChange(newLanguages, newPrimary)
  }

  const handlePrimaryChange = (languageCode: string) => {
    if (disabled || !selectedLanguages.includes(languageCode)) return
    onLanguagesChange(selectedLanguages, languageCode)
  }

  const removeLanguage = (languageCode: string) => {
    handleLanguageToggle(languageCode)
  }

  const getLanguageName = (code: string) => {
    const language = availableLanguages.find(lang => lang.code === code)
    return language?.name || code
  }

  return (
    <div className={`relative ${className}`}>
      {/* Selected Languages Display */}
      <div className="mb-3">
        <label className="block text-sm font-medium text-gray-700 mb-2">
          OCR Languages {selectedLanguages.length > 0 && `(${selectedLanguages.length}/${maxLanguages})`}
        </label>
        
        {selectedLanguages.length > 0 ? (
          <div className="flex flex-wrap gap-2">
            {selectedLanguages.map((langCode) => (
              <span
                key={langCode}
                className={`inline-flex items-center px-3 py-1 rounded-full text-sm font-medium ${
                  langCode === effectivePrimary
                    ? 'bg-blue-100 text-blue-800 border-2 border-blue-300'
                    : 'bg-gray-100 text-gray-800 border border-gray-300'
                }`}
              >
                {getLanguageName(langCode)}
                {langCode === effectivePrimary && (
                  <span className="ml-1 text-xs font-bold text-blue-600">(Primary)</span>
                )}
                {!disabled && (
                  <button
                    type="button"
                    onClick={() => removeLanguage(langCode)}
                    className="ml-2 text-gray-400 hover:text-gray-600"
                  >
                    <XMarkIcon className="h-4 w-4" />
                  </button>
                )}
              </span>
            ))}
          </div>
        ) : (
          <div className="text-sm text-gray-500 italic">
            No languages selected. Documents will use default OCR language.
          </div>
        )}
      </div>

      {/* Language Selector Button */}
      {!disabled && (
        <button
          type="button"
          onClick={() => setIsOpen(!isOpen)}
          className="w-full px-4 py-2 text-left border border-gray-300 rounded-lg bg-white hover:bg-gray-50 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        >
          <span className="text-gray-600">
            {selectedLanguages.length === 0 
              ? 'Select OCR languages...' 
              : `Add more languages (${maxLanguages - selectedLanguages.length} remaining)`
            }
          </span>
        </button>
      )}

      {/* Dropdown Panel */}
      {isOpen && !disabled && (
        <div className="absolute z-10 mt-1 w-full bg-white border border-gray-300 rounded-lg shadow-lg max-h-64 overflow-y-auto">
          <div className="p-3">
            <div className="text-xs text-gray-500 mb-2 uppercase tracking-wide font-semibold">
              Available Languages
            </div>
            
            <div className="space-y-1">
              {availableLanguages
                .filter(lang => lang.installed)
                .map((language) => {
                  const isSelected = selectedLanguages.includes(language.code)
                  const isPrimary = language.code === effectivePrimary
                  const canSelect = !isSelected && selectedLanguages.length < maxLanguages
                  
                  return (
                    <div
                      key={language.code}
                      className={`flex items-center justify-between p-2 rounded ${
                        isSelected 
                          ? 'bg-blue-50 border border-blue-200' 
                          : canSelect 
                            ? 'hover:bg-gray-50 cursor-pointer' 
                            : 'opacity-50 cursor-not-allowed'
                      }`}
                    >
                      <div className="flex items-center">
                        <button
                          type="button"
                          onClick={() => handleLanguageToggle(language.code)}
                          disabled={!canSelect && !isSelected}
                          className={`flex items-center space-x-2 ${
                            canSelect || isSelected ? 'cursor-pointer' : 'cursor-not-allowed'
                          }`}
                        >
                          <div className={`w-5 h-5 border-2 rounded flex items-center justify-center ${
                            isSelected 
                              ? 'border-blue-500 bg-blue-500' 
                              : 'border-gray-300'
                          }`}>
                            {isSelected && <CheckIcon className="h-3 w-3 text-white" />}
                          </div>
                          <span className={`text-sm ${isSelected ? 'font-medium text-blue-900' : 'text-gray-700'}`}>
                            {language.name}
                          </span>
                          {isPrimary && (
                            <span className="text-xs bg-blue-600 text-white px-2 py-0.5 rounded font-bold">
                              PRIMARY
                            </span>
                          )}
                        </button>
                      </div>
                      
                      {/* Primary selector */}
                      {isSelected && showPrimarySelector && selectedLanguages.length > 1 && (
                        <button
                          type="button"
                          onClick={() => handlePrimaryChange(language.code)}
                          className={`text-xs px-2 py-1 rounded font-medium ${
                            isPrimary 
                              ? 'bg-blue-600 text-white cursor-default' 
                              : 'bg-gray-200 text-gray-700 hover:bg-gray-300'
                          }`}
                        >
                          {isPrimary ? 'Primary' : 'Set Primary'}
                        </button>
                      )}
                    </div>
                  )
                })}
            </div>
            
            {selectedLanguages.length >= maxLanguages && (
              <div className="mt-3 p-2 bg-amber-50 border border-amber-200 rounded text-xs text-amber-800">
                Maximum {maxLanguages} languages allowed for optimal performance.
              </div>
            )}
          </div>
          
          <div className="border-t border-gray-200 p-3">
            <button
              type="button"
              onClick={() => setIsOpen(false)}
              className="w-full text-center text-sm text-gray-600 hover:text-gray-800"
            >
              Close
            </button>
          </div>
        </div>
      )}

      {/* Help Text */}
      {selectedLanguages.length > 1 && (
        <div className="mt-2 text-xs text-gray-500">
          <p>
            <strong>Primary language</strong> is processed first for better accuracy. 
            Multiple languages help with mixed-language documents.
          </p>
        </div>
      )}
    </div>
  )
}

export default LanguageSelector