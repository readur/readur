import React, { useState, useEffect, useRef } from 'react'
import { CheckIcon, XMarkIcon } from '@heroicons/react/24/outline'
import { ocrService, LanguageInfo } from '../../services/api'
import { useTheme } from '@mui/material/styles'
import { Box, Typography, Chip, Button, Paper, Divider, Popper, ClickAwayListener, CircularProgress, Alert } from '@mui/material'

interface LanguageSelectorProps {
  selectedLanguages: string[]
  primaryLanguage?: string
  onLanguagesChange: (languages: string[], primary?: string) => void
  maxLanguages?: number
  disabled?: boolean
  showPrimarySelector?: boolean
  className?: string
}

function LanguageSelector({
  selectedLanguages,
  primaryLanguage,
  onLanguagesChange,
  maxLanguages = 4,
  disabled = false,
  showPrimarySelector = true,
  className = '',
}: LanguageSelectorProps) {
  const theme = useTheme()
  const [availableLanguages, setAvailableLanguages] = useState<LanguageInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string>('')
  const [isOpen, setIsOpen] = useState(false)
  const anchorRef = useRef<HTMLButtonElement>(null)

  useEffect(() => {
    fetchLanguages()
  }, [])

  const fetchLanguages = async () => {
    try {
      setLoading(true)
      setError('')
      const response = await ocrService.getAvailableLanguages()
      setAvailableLanguages(response.data.available_languages)
    } catch {
      setError('Failed to load languages')
      // Fallback to English only if API fails
      setAvailableLanguages([{ code: 'eng', name: 'English', installed: true }])
    } finally {
      setLoading(false)
    }
  }

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

  const handleClose = () => {
    setIsOpen(false)
  }

  const getLanguageName = (code: string) => {
    const language = availableLanguages.find(lang => lang.code === code)
    return language?.name || code
  }

  return (
    <Box sx={{ position: 'relative' }} className={className}>
      {/* Selected Languages Display */}
      <Box sx={{ mb: 3 }}>
        <Typography variant="body2" sx={{ 
          fontWeight: 500, 
          color: 'text.primary', 
          mb: 2 
        }}>
          OCR Languages {selectedLanguages.length > 0 && `(${selectedLanguages.length}/${maxLanguages})`}
        </Typography>
        
        {selectedLanguages.length > 0 ? (
          <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
            {selectedLanguages.map((langCode) => (
              <Chip
                key={langCode}
                label={
                  <Box sx={{ display: 'flex', alignItems: 'center' }}>
                    <span>{getLanguageName(langCode)}</span>
                    {langCode === effectivePrimary && (
                      <Typography variant="caption" sx={{ 
                        ml: 1, 
                        fontWeight: 'bold',
                        color: 'primary.main'
                      }}>
                        (Primary)
                      </Typography>
                    )}
                  </Box>
                }
                variant={langCode === effectivePrimary ? 'filled' : 'outlined'}
                color={langCode === effectivePrimary ? 'primary' : 'default'}
                size="small"
                onDelete={!disabled ? () => removeLanguage(langCode) : undefined}
                deleteIcon={<XMarkIcon style={{ width: 16, height: 16 }} />}
                sx={{
                  '& .MuiChip-deleteIcon': {
                    color: 'text.secondary',
                    '&:hover': {
                      color: 'text.primary',
                    },
                  },
                }}
              />
            ))}
          </Box>
        ) : (
          <Typography variant="body2" sx={{ 
            color: 'text.secondary', 
            fontStyle: 'italic' 
          }}>
            No languages selected. Documents will use default OCR language.
          </Typography>
        )}
      </Box>

      {/* Language Selector Button */}
      {!disabled && (
        <Button
          ref={anchorRef}
          variant="outlined"
          onClick={() => setIsOpen(!isOpen)}
          fullWidth
          sx={{
            justifyContent: 'flex-start',
            textTransform: 'none',
            color: 'text.secondary',
            borderColor: 'divider',
            py: 1,
            backgroundColor: (theme) => theme.palette.mode === 'dark' 
              ? 'rgba(255, 255, 255, 0.02)'
              : 'rgba(0, 0, 0, 0.02)',
            '&:hover': {
              backgroundColor: (theme) => theme.palette.mode === 'dark'
                ? 'rgba(255, 255, 255, 0.05)'
                : 'rgba(0, 0, 0, 0.04)',
              borderColor: 'divider',
            },
          }}
        >
          {selectedLanguages.length === 0 
            ? 'Select OCR languages...' 
            : `Add more languages (${maxLanguages - selectedLanguages.length} remaining)`
          }
        </Button>
      )}

      {/* Dropdown Panel */}
      <Popper
        open={isOpen && !disabled}
        anchorEl={anchorRef.current}
        placement="bottom-start"
        sx={{ zIndex: 1300 }}
        modifiers={[
          {
            name: 'offset',
            options: {
              offset: [0, 8],
            },
          },
        ]}
      >
        <ClickAwayListener onClickAway={handleClose}>
          <Paper
            elevation={8}
            sx={{
              width: anchorRef.current?.offsetWidth || 300,
              maxWidth: 500,
              maxHeight: '60vh',
              overflow: 'auto',
              borderRadius: 2,
              '&::-webkit-scrollbar': {
                width: '6px',
              },
              '&::-webkit-scrollbar-track': {
                background: 'transparent',
              },
              '&::-webkit-scrollbar-thumb': {
                background: 'divider',
                borderRadius: '3px',
                '&:hover': {
                  background: 'text.disabled',
                },
              },
            }}
          >
          <Box sx={{ p: 3 }}>
            <Typography variant="subtitle2" sx={{
              color: 'text.secondary',
              mb: 2,
              textTransform: 'uppercase',
              letterSpacing: 1,
              fontWeight: 600,
              fontSize: '0.75rem'
            }}>
              Available Languages
            </Typography>

            {loading && (
              <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', py: 4 }}>
                <CircularProgress size={24} sx={{ mr: 2 }} />
                <Typography variant="body2" color="text.secondary">
                  Loading languages...
                </Typography>
              </Box>
            )}

            {error && (
              <Alert severity="warning" sx={{ mb: 2 }}>
                {error}
              </Alert>
            )}

            {!loading && (
            <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
              {availableLanguages
                .filter(lang => lang.installed)
                .map((language) => {
                  const isSelected = selectedLanguages.includes(language.code)
                  const isPrimary = language.code === effectivePrimary
                  const canSelect = !isSelected && selectedLanguages.length < maxLanguages
                  
                  return (
                    <Box
                      key={language.code}
                      sx={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        p: 2,
                        borderRadius: 1.5,
                        backgroundColor: isSelected 
                          ? (theme) => theme.palette.mode === 'dark' 
                              ? 'rgba(144, 202, 249, 0.08)' 
                              : 'rgba(25, 118, 210, 0.08)'
                          : 'transparent',
                        cursor: canSelect || isSelected ? 'pointer' : 'not-allowed',
                        opacity: !canSelect && !isSelected ? 0.5 : 1,
                        transition: 'all 0.2s ease-in-out',
                        '&:hover': canSelect ? {
                          backgroundColor: (theme) => theme.palette.mode === 'dark'
                            ? 'rgba(255, 255, 255, 0.05)'
                            : 'rgba(0, 0, 0, 0.04)',
                          transform: 'translateY(-1px)',
                        } : {},
                      }}
                    >
                      <Box 
                        sx={{ 
                          display: 'flex', 
                          alignItems: 'center',
                          cursor: canSelect || isSelected ? 'pointer' : 'not-allowed',
                          gap: 2,
                        }}
                        onClick={() => canSelect || isSelected ? handleLanguageToggle(language.code) : undefined}
                      >
                        <Box
                          sx={{
                            width: 22,
                            height: 22,
                            border: 2,
                            borderRadius: 1,
                            borderColor: isSelected ? 'primary.main' : 'divider',
                            backgroundColor: isSelected ? 'primary.main' : 'transparent',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            transition: 'all 0.15s ease-in-out',
                            '&:hover': canSelect && !isSelected ? {
                              borderColor: 'primary.light',
                            } : {},
                          }}
                        >
                          {isSelected && (
                            <CheckIcon 
                              style={{ 
                                width: 14, 
                                height: 14, 
                                color: theme.palette.primary.contrastText,
                              }} 
                            />
                          )}
                        </Box>
                        <Typography
                          variant="body2"
                          sx={{
                            fontWeight: isSelected ? 500 : 400,
                            color: isSelected ? 'primary.main' : 'text.primary',
                          }}
                        >
                          {language.name}
                        </Typography>
                        {isPrimary && (
                          <Chip
                            label="PRIMARY"
                            size="small"
                            color="primary"
                            sx={{
                              height: 20,
                              fontSize: '0.65rem',
                              fontWeight: 'bold',
                            }}
                          />
                        )}
                      </Box>
                      
                      {/* Primary selector */}
                      {isSelected && showPrimarySelector && selectedLanguages.length > 1 && (
                        <Button
                          size="small"
                          variant={isPrimary ? "contained" : "text"}
                          color="primary"
                          onClick={() => handlePrimaryChange(language.code)}
                          disabled={isPrimary}
                          sx={{
                            fontSize: '0.7rem',
                            py: 0.5,
                            px: 1,
                            minWidth: 'auto',
                            textTransform: 'none',
                            opacity: isPrimary ? 0.7 : 1,
                          }}
                        >
                          {isPrimary ? 'Primary' : 'Set Primary'}
                        </Button>
                      )}
                    </Box>
                  )
                })}
            </Box>
            )}

            {selectedLanguages.length >= maxLanguages && (
              <Box sx={{ 
                mt: 3, 
                p: 2, 
                backgroundColor: (theme) => theme.palette.mode === 'dark'
                  ? 'rgba(255, 193, 7, 0.08)'
                  : 'rgba(255, 193, 7, 0.08)',
                border: '1px solid', 
                borderColor: (theme) => theme.palette.mode === 'dark'
                  ? 'rgba(255, 193, 7, 0.3)'
                  : 'rgba(255, 193, 7, 0.3)',
                borderRadius: 2,
              }}>
                <Typography variant="body2" sx={{ 
                  color: 'warning.main',
                  fontWeight: 500,
                }}>
                  Maximum {maxLanguages} languages allowed for optimal performance.
                </Typography>
              </Box>
            )}
          </Box>
          
          <Divider sx={{ borderColor: 'divider' }} />
          <Box sx={{ p: 2.5 }}>
            <Button
              variant="text"
              onClick={handleClose}
              fullWidth
              sx={{
                textTransform: 'none',
                color: 'text.secondary',
                py: 1.5,
                fontSize: '0.875rem',
                fontWeight: 500,
                '&:hover': {
                  color: 'text.primary',
                  backgroundColor: 'action.hover',
                },
              }}
            >
              Close
            </Button>
          </Box>
          </Paper>
        </ClickAwayListener>
      </Popper>

      {/* Help Text */}
      {selectedLanguages.length > 1 && (
        <Box sx={{ mt: 2 }}>
          <Typography variant="caption" sx={{ color: 'text.secondary' }}>
            <strong>Primary language</strong> is processed first for better accuracy. 
            Multiple languages help with mixed-language documents.
          </Typography>
        </Box>
      )}
    </Box>
  )
}

export default LanguageSelector