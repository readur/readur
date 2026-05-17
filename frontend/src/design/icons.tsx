/**
 * Lucide icon adapter for Readur.
 *
 * The codebase originally imported icons from `@mui/icons-material` and
 * `@heroicons/react/24/outline`. This module re-exports their Lucide
 * equivalents under the same names so that call-sites only need to swap
 * their import path:
 *
 *   - import { Menu, Dashboard } from './icons'
 *   + import { Menu, Dashboard } from '../design/icons'
 *
 * Each exported icon is wrapped in `make()` so that MUI-style props
 * (`fontSize="small"`, `sx={{ fontSize: 18 }}`) translate into a Lucide
 * `size` value. This keeps call-sites visually consistent with their
 * previous MUI behaviour.
 */
import React, { forwardRef, type CSSProperties } from 'react';
import * as L from 'lucide-react';
import type { LucideProps } from 'lucide-react';

type FontSize = 'small' | 'medium' | 'large' | 'inherit' | number | string;

interface IconProps extends Omit<LucideProps, 'size'> {
  /** MUI-compatible fontSize prop. */
  fontSize?: FontSize;
  /** Sx-style overrides — only `fontSize` is interpreted here. */
  sx?: { fontSize?: number | string;[k: string]: unknown };
  /** Plain HTML style — applied last. */
  style?: CSSProperties;
}

const resolveSize = (fontSize?: FontSize, sxFontSize?: number | string): number | string => {
  if (typeof sxFontSize === 'number') return sxFontSize;
  if (typeof sxFontSize === 'string') return sxFontSize;
  if (fontSize === 'small') return 18;
  if (fontSize === 'medium') return 22;
  if (fontSize === 'large') return 32;
  if (fontSize === 'inherit') return '1em';
  if (typeof fontSize === 'number') return fontSize;
  if (typeof fontSize === 'string') return fontSize;
  return 20;
};

type IconComponent = React.ForwardRefExoticComponent<
  IconProps & React.RefAttributes<SVGSVGElement>
>;

const make = (Lu: React.FC<LucideProps>, displayName: string): IconComponent => {
  // MUI's IconButton/icon auto-tests use `data-testid="${name}Icon"`. We
  // preserve that contract so existing test assertions still find the icon
  // by `screen.getByTestId('RefreshIcon')` etc.
  const testId = displayName.endsWith('Icon') ? displayName : `${displayName}Icon`;
  const Component = forwardRef<SVGSVGElement, IconProps>(
    ({ fontSize, sx, style, color, ...rest }, ref) => {
      const size = resolveSize(fontSize, sx?.fontSize);
      const sxOther = sx
        ? Object.fromEntries(Object.entries(sx).filter(([k]) => k !== 'fontSize'))
        : undefined;
      const mergedStyle: CSSProperties = {
        verticalAlign: 'middle',
        flexShrink: 0,
        ...(sxOther as CSSProperties | undefined),
        ...style,
      };
      // `color` prop is MUI's semantic ("primary", "error", etc.) or a CSS color.
      // For Lucide we just let `currentColor` from CSS take over unless an
      // explicit CSS color was passed.
      const colorProp =
        color && typeof color === 'string' && color.startsWith('#') ? color : 'currentColor';
      const restProps = rest as LucideProps & { 'data-testid'?: string };
      return (
        <Lu
          ref={ref as any}
          size={size as number | string}
          strokeWidth={1.5}
          color={colorProp}
          style={mergedStyle}
          data-testid={restProps['data-testid'] ?? testId}
          {...(rest as LucideProps)}
        />
      );
    },
  );
  Component.displayName = displayName;
  return Component;
};

// ---------------------------------------------------------------------------
// MUI icon name → Lucide component
// 132 unique icons used across the codebase (see Phase 6 migration notes).
// ---------------------------------------------------------------------------
export const AccessTime = make(L.Clock, 'AccessTime');
export const AccountCircle = make(L.CircleUser, 'AccountCircle');
export const Add = make(L.Plus, 'Add');
export const AdminPanelSettings = make(L.ShieldCheck, 'AdminPanelSettings');
export const Api = make(L.Code2, 'Api');
export const Archive = make(L.Archive, 'Archive');
export const ArrowBack = make(L.ArrowLeft, 'ArrowBack');
export const Article = make(L.Newspaper, 'Article');
export const AspectRatio = make(L.RectangleHorizontal, 'AspectRatio');
export const Assessment = make(L.BarChart3, 'Assessment');
export const Assignment = make(L.ClipboardList, 'Assignment');
export const AttachMoney = make(L.DollarSign, 'AttachMoney');
export const AutoFixHigh = make(L.Wand2, 'AutoFixHigh');
export const Block = make(L.Ban, 'Block');
export const BlurOn = make(L.CircleDot, 'BlurOn');
export const BookmarkBorder = make(L.Bookmark, 'BookmarkBorder');
export const Brightness4 = make(L.Moon, 'Brightness4');
export const Brightness7 = make(L.Sun, 'Brightness7');
export const BugReport = make(L.Bug, 'BugReport');
export const Build = make(L.Wrench, 'Build');
export const Business = make(L.Building2, 'Business');
export const BusinessCenter = make(L.Briefcase, 'BusinessCenter');
export const CalendarToday = make(L.Calendar, 'CalendarToday');
export const ChatBubbleOutline = make(L.MessageCircle, 'ChatBubbleOutline');
export const Check = make(L.Check, 'Check');
export const CheckBox = make(L.SquareCheck, 'CheckBox');
export const CheckBoxOutlineBlank = make(L.Square, 'CheckBoxOutlineBlank');
export const CheckCircle = make(L.CircleCheck, 'CheckCircle');
export const CheckCircleOutline = make(L.CircleCheck, 'CheckCircleOutline');
export const ChevronLeft = make(L.ChevronLeft, 'ChevronLeft');
export const ChevronRight = make(L.ChevronRight, 'ChevronRight');
export const Clear = make(L.X, 'Clear');
export const Close = make(L.X, 'Close');
export const Cloud = make(L.Cloud, 'Cloud');
export const CloudOff = make(L.CloudOff, 'CloudOff');
export const CloudSync = make(L.RefreshCw, 'CloudSync');
export const CloudUpload = make(L.UploadCloud, 'CloudUpload');
export const Code = make(L.Code2, 'Code');
export const ColorLens = make(L.Palette, 'ColorLens');
export const Computer = make(L.Monitor, 'Computer');
export const ContentCopy = make(L.Copy, 'ContentCopy');
export const Copyright = make(L.Copyright, 'Copyright');
export const CreateNewFolder = make(L.FolderPlus, 'CreateNewFolder');
export const Dashboard = make(L.LayoutDashboard, 'Dashboard');
export const DateRange = make(L.CalendarDays, 'DateRange');
export const Delete = make(L.Trash2, 'Delete');
export const Description = make(L.FileText, 'Description');
export const DoneAll = make(L.CheckCheck, 'DoneAll');
export const Download = make(L.Download, 'Download');
export const Edit = make(L.Pencil, 'Edit');
export const Error = make(L.CircleAlert, 'Error');
export const ExpandLess = make(L.ChevronUp, 'ExpandLess');
export const ExpandMore = make(L.ChevronDown, 'ExpandMore');
export const Extension = make(L.Puzzle, 'Extension');
export const FileCopy = make(L.Files, 'FileCopy');
export const FilterList = make(L.Filter, 'FilterList');
export const FindInPage = make(L.FileSearch, 'FindInPage');
export const Fingerprint = make(L.Fingerprint, 'Fingerprint');
export const Folder = make(L.Folder, 'Folder');
export const FormatQuote = make(L.Quote, 'FormatQuote');
export const FormatSize = make(L.Type, 'FormatSize');
export const Functions = make(L.Sigma, 'Functions');
export const GetApp = make(L.Download, 'GetApp');
export const GridView = make(L.LayoutGrid, 'GridView');
export const Group = make(L.Users, 'Group');
export const HealthAndSafety = make(L.ShieldCheck, 'HealthAndSafety');
export const Help = make(L.CircleHelp, 'Help');
export const History = make(L.History, 'History');
export const Image = make(L.Image, 'Image');
export const Info = make(L.Info, 'Info');
export const InsertDriveFile = make(L.File, 'InsertDriveFile');
export const Key = make(L.Key, 'Key');
export const Label = make(L.Tag, 'Label');
export const Language = make(L.Globe, 'Language');
export const Lightbulb = make(L.Lightbulb, 'Lightbulb');
export const LinkOff = make(L.Unlink, 'LinkOff');
export const LocalHospital = make(L.CrossIcon ?? L.Plus, 'LocalHospital');
export const LocationOn = make(L.MapPin, 'LocationOn');
export const Lock = make(L.Lock, 'Lock');
export const Logout = make(L.LogOut, 'Logout');
export const ManageAccounts = make(L.UserCog, 'ManageAccounts');
export const ManageSearch = make(L.SearchCheck, 'ManageSearch');
export const Menu = make(L.Menu, 'Menu');
export const MoreVert = make(L.MoreVertical, 'MoreVert');
export const NetworkCheck = make(L.Activity, 'NetworkCheck');
export const Notifications = make(L.Bell, 'Notifications');
export const OpenInNew = make(L.ExternalLink, 'OpenInNew');
export const Pause = make(L.Pause, 'Pause');
export const Pending = make(L.Clock, 'Pending');
export const Person = make(L.User, 'Person');
export const PersonOutline = make(L.User, 'PersonOutline');
export const PhotoCamera = make(L.Camera, 'PhotoCamera');
export const PhotoFilter = make(L.Sparkles, 'PhotoFilter');
export const PictureAsPdf = make(L.FileType2, 'PictureAsPdf');
export const PlayArrow = make(L.Play, 'PlayArrow');
export const PriorityHigh = make(L.AlertCircle, 'PriorityHigh');
export const Psychology = make(L.Brain, 'Psychology');
export const Receipt = make(L.Receipt, 'Receipt');
export const Refresh = make(L.RefreshCw, 'Refresh');
export const RemoveCircle = make(L.CircleMinus, 'RemoveCircle');
export const Reply = make(L.Reply, 'Reply');
export const RestoreFromTrash = make(L.ArchiveRestore, 'RestoreFromTrash');
export const Scale = make(L.Scale, 'Scale');
export const Schedule = make(L.Clock, 'Schedule');
export const Search = make(L.Search, 'Search');
export const Security = make(L.Shield, 'Security');
export const SelectAll = make(L.SquareCheck, 'SelectAll');
export const Send = make(L.Send, 'Send');
export const Settings = make(L.Settings, 'Settings');
export const Share = make(L.Share2, 'Share');
export const Sort = make(L.ArrowUpDown, 'Sort');
export const Speed = make(L.Gauge, 'Speed');
export const Star = make(L.Star, 'Star');
export const Stop = make(L.Square, 'Stop');
export const Storage = make(L.Database, 'Storage');
export const Sync = make(L.RefreshCw, 'Sync');
export const TableChart = make(L.Table2, 'TableChart');
export const Tag = make(L.Tag, 'Tag');
export const TextFormat = make(L.Type, 'TextFormat');
export const TextSnippet = make(L.FileText, 'TextSnippet');
export const Timeline = make(L.Activity, 'Timeline');
export const Timer = make(L.Timer, 'Timer');
export const TrendingUp = make(L.TrendingUp, 'TrendingUp');
export const Tune = make(L.SlidersHorizontal, 'Tune');
export const Upload = make(L.Upload, 'Upload');
export const ViewList = make(L.List, 'ViewList');
export const ViewModule = make(L.LayoutGrid, 'ViewModule');
export const Visibility = make(L.Eye, 'Visibility');
export const VisibilityOff = make(L.EyeOff, 'VisibilityOff');
export const Warning = make(L.TriangleAlert, 'Warning');
export const Work = make(L.Briefcase, 'Work');
export const WrapText = make(L.WrapText, 'WrapText');

// ---------------------------------------------------------------------------
// Heroicons name → Lucide component
// ---------------------------------------------------------------------------
export const ArrowDownTrayIcon = make(L.Download, 'ArrowDownTrayIcon');
export const ChartBarIcon = make(L.BarChart3, 'ChartBarIcon');
export const CheckIcon = make(L.Check, 'CheckIcon');
export const ClockIcon = make(L.Clock, 'ClockIcon');
export const DocumentArrowUpIcon = make(L.FileUp, 'DocumentArrowUpIcon');
export const DocumentIcon = make(L.File, 'DocumentIcon');
export const DocumentTextIcon = make(L.FileText, 'DocumentTextIcon');
export const ExclamationCircleIcon = make(L.CircleAlert, 'ExclamationCircleIcon');
export const MagnifyingGlassIcon = make(L.Search, 'MagnifyingGlassIcon');
export const PhotoIcon = make(L.Image, 'PhotoIcon');
export const XMarkIcon = make(L.X, 'XMarkIcon');
